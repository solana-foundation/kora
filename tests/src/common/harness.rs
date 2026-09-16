#![cfg(test)]

use anyhow::{anyhow, Context, Result};
use solana_sdk::pubkey::Pubkey;
use std::{
    fs,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Stdio,
    str::FromStr,
    sync::{
        atomic::{AtomicI32, Ordering},
        OnceLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use surfpool_sdk::{cheatcodes::builders::DeployProgram, BlockProductionMode, Surfnet};
use tokio::{
    process::{Child, Command},
    sync::OnceCell,
};

use crate::common::{
    client::TestContext,
    constants::{
        DISABLE_SBPF_V0_V1_V2_DEPLOYMENT_FEATURE, KORA_PRIVATE_KEY_ENV, LIGHTHOUSE_PROGRAM_ID,
        LIGHTHOUSE_PROGRAM_PATH, PAYMENT_ADDRESS_KEYPAIR_ENV, RPC_URL_ENV, SIGNER_2_KEYPAIR_ENV,
        TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV, TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
        TEST_FEE_PAYER_POLICY_MINT_2022_KEYPAIR_ENV, TEST_FEE_PAYER_POLICY_MINT_KEYPAIR_ENV,
        TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV, TEST_SENDER_KEYPAIR_ENV,
        TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV, TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV,
        TEST_USDC_MINT_2022_KEYPAIR_ENV, TEST_USDC_MINT_KEYPAIR_ENV, TRANSFER_HOOK_PROGRAM_ID,
        TRANSFER_HOOK_PROGRAM_PATH,
    },
    seed::{seed_accounts, SeededLookupTables},
};

/// Kora reads each signing key from the environment, so the fixtures under
/// `local-keys/` have to be exported before the node starts.
const LOCAL_KEY_ENV_FILES: &[(&str, &str)] = &[
    (KORA_PRIVATE_KEY_ENV, "fee-payer-local.json"),
    (SIGNER_2_KEYPAIR_ENV, "signer2-local.json"),
    (TEST_SENDER_KEYPAIR_ENV, "sender-local.json"),
    (TEST_USDC_MINT_KEYPAIR_ENV, "usdc-mint-local.json"),
    (TEST_USDC_MINT_2022_KEYPAIR_ENV, "usdc-mint-2022-local.json"),
    (TEST_FEE_PAYER_POLICY_MINT_KEYPAIR_ENV, "fee-payer-policy-mint-local.json"),
    (TEST_FEE_PAYER_POLICY_MINT_2022_KEYPAIR_ENV, "fee-payer-policy-mint-local-2022.json"),
    (TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV, "mint-2022-interest-bearing.json"),
    (TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV, "mint-transfer-hook-local.json"),
    (PAYMENT_ADDRESS_KEYPAIR_ENV, "payment-local.json"),
];

const KORA_BINARY_PATH_ENV: &str = "KORA_TEST_BINARY_PATH";
const KORA_BINARY_PATH: &str = "target/debug/kora";
const SLOT_TIME_MS: u64 = 400;
const SURFNET_START_ATTEMPTS: usize = 5;

static KORA_PID: AtomicI32 = AtomicI32::new(0);
static RENDERED_CONFIG: OnceLock<PathBuf> = OnceLock::new();
static HARNESS: OnceCell<KoraHarness> = OnceCell::const_new();

/// Paths are workspace-relative.
#[derive(Clone, Copy)]
pub struct KoraSpec {
    pub config: &'static str,
    pub signers: &'static str,
    pub initialize_payments_atas: bool,
}

/// A surfnet plus the Kora node pointed at it, seeded from scratch.
pub struct KoraHarness {
    _surfnet: Surfnet,
    _kora: Child,
    pub server_url: String,
    pub rpc_url: String,
}

impl KoraHarness {
    pub async fn start(spec: KoraSpec) -> Result<Self> {
        let (surfnet, lookup_tables) = start_surfnet_with_retry().await?;

        let rpc_url = surfnet.rpc_url().to_string();
        std::env::set_var(RPC_URL_ENV, &rpc_url);

        for (env_var, address) in [
            (TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV, lookup_tables.allowed),
            (TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV, lookup_tables.disallowed),
            (TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV, lookup_tables.transaction),
        ] {
            std::env::set_var(env_var, address.to_string());
        }

        let config_path = render_config(&workspace_path(spec.config), &rpc_url)?;
        let signers_path = workspace_path(spec.signers);

        if spec.initialize_payments_atas {
            initialize_payment_atas(&config_path, &signers_path, &rpc_url).await?;
        }

        let port = free_port()?;
        let mut kora = spawn_kora(&config_path, &signers_path, &rpc_url, port)?;
        register_kora_teardown(&kora)?;
        wait_for_liveness(&mut kora, port).await?;

        let server_url = format!("http://127.0.0.1:{port}");

        Ok(Self { _surfnet: surfnet, _kora: kora, server_url, rpc_url })
    }
}

/// Kora keeps its config in process-global state, so one node per config means
/// one harness per test binary. Each binary compiles its own `common`, and so
/// its own `HARNESS`.
///
/// Contexts are rebuilt per test rather than shared: every `#[tokio::test]` owns
/// its runtime, and a client outliving that runtime loses its dispatch task.
pub async fn harness_context(spec: KoraSpec) -> TestContext {
    let harness = HARNESS
        .get_or_init(|| async {
            KoraHarness::start(spec).await.expect("Failed to start Kora harness")
        })
        .await;

    TestContext::with_urls(harness.server_url.clone(), harness.rpc_url.clone())
        .await
        .expect("Failed to create test context")
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("tests/ always has a parent")
}

fn workspace_path(relative: &str) -> PathBuf {
    workspace_root().join(relative)
}

/// Test helpers read these before touching the harness, so they must be set
/// before any test body runs.
#[ctor::ctor]
fn set_local_key_env_vars() {
    for (env_var, filename) in LOCAL_KEY_ENV_FILES {
        let key = read_local_key(filename).expect("failed to read local test keypair");
        std::env::set_var(env_var, key);
    }
}

/// The SDK picks its two ports by binding an ephemeral listener and dropping it
/// before the servers bind for real, so a port can be taken in between, either
/// by the SDK's own second probe or by another test binary starting at the same
/// time. The allocation is fresh per attempt, so retrying clears it.
async fn start_surfnet_with_retry() -> Result<(Surfnet, SeededLookupTables)> {
    let mut last_error = None;
    for _ in 0..SURFNET_START_ATTEMPTS {
        let attempt = tokio::task::spawn_blocking(|| std::thread::spawn(start_surfnet).join())
            .await?
            .map_err(|_| anyhow!("surfnet startup thread panicked"))?;
        match attempt {
            Ok(started) => return Ok(started),
            Err(e) if e.to_string().contains("Address already in use") => last_error = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_error
        .unwrap_or_else(|| anyhow!("surfnet startup failed"))
        .context(format!("surfnet did not start in {SURFNET_START_ATTEMPTS} attempts")))
}

/// The surfpool SDK and its cheatcodes drive the blocking Solana RPC client,
/// which cannot run under the current-thread runtime `#[tokio::test]` builds.
fn start_surfnet() -> Result<(Surfnet, SeededLookupTables)> {
    let runtime =
        tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build()?;
    runtime.block_on(async {
        let surfnet = Surfnet::builder()
            .block_production_mode(BlockProductionMode::Clock)
            .slot_time_ms(SLOT_TIME_MS)
            .disable_feature(Pubkey::from_str(DISABLE_SBPF_V0_V1_V2_DEPLOYMENT_FEATURE)?)
            .start()
            .await
            .map_err(|e| anyhow!("failed to start surfnet: {e}"))?;
        deploy_test_programs(&surfnet)?;
        let lookup_tables = seed_accounts(&surfnet)?;
        Ok((surfnet, lookup_tables))
    })
}

fn deploy_test_programs(surfnet: &Surfnet) -> Result<()> {
    let cheats = surfnet.cheatcodes();
    for (program_id, program_path) in [
        (TRANSFER_HOOK_PROGRAM_ID, TRANSFER_HOOK_PROGRAM_PATH),
        (LIGHTHOUSE_PROGRAM_ID, LIGHTHOUSE_PROGRAM_PATH),
    ] {
        let path = workspace_path(program_path);
        if !path.exists() {
            return Err(anyhow!("test program not found at {}", path.display()));
        }
        cheats
            .deploy(DeployProgram::new(Pubkey::from_str(program_id)?).so_path(path))
            .map_err(|e| anyhow!("failed to deploy {program_id}: {e}"))?;
    }
    Ok(())
}

/// Kora's config loader does no environment interpolation, so the fixture is
/// rewritten per run with the surfnet URL as the Jito endpoint.
fn render_config(source: &Path, rpc_url: &str) -> Result<PathBuf> {
    let contents = fs::read_to_string(source)
        .with_context(|| format!("failed to read config {}", source.display()))?;
    let mut doc: toml::Table = toml::from_str(&contents)?;

    if let Some(jito) = doc
        .get_mut("kora")
        .and_then(|kora| kora.get_mut("bundle"))
        .and_then(|bundle| bundle.get_mut("jito"))
        .and_then(toml::Value::as_table_mut)
    {
        jito.insert("block_engine_url".to_string(), rpc_url.into());
        jito.insert("simulate_bundle_url".to_string(), rpc_url.into());
    }

    let file_stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("kora");
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path =
        std::env::temp_dir().join(format!("{file_stem}-{}-{unique}.toml", std::process::id()));

    // create_new refuses to follow a symlink another process left at this path
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
    file.write_all(toml::to_string(&doc)?.as_bytes())?;
    let _ = RENDERED_CONFIG.set(path.clone());
    Ok(path)
}

fn kora_binary_path() -> Result<PathBuf> {
    let path = std::env::var(KORA_BINARY_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_path(KORA_BINARY_PATH));
    if !path.exists() {
        return Err(anyhow!(
            "pre-built Kora binary not found at '{}'. Run 'cargo build --bin kora' first.",
            path.display()
        ));
    }
    Ok(path)
}

fn kora_command(config: &Path, rpc_url: &str) -> Result<Command> {
    let mut cmd = Command::new(kora_binary_path()?);
    cmd.arg("--config")
        .arg(config)
        .arg("--rpc-url")
        .arg(rpc_url)
        .env("KORA_PRIVATE_KEY", read_local_key("fee-payer-local.json")?)
        .env("KORA_PRIVATE_KEY_2", read_local_key("signer2-local.json")?)
        .env_remove("KORA_REDIS_URL");

    if let Ok(jupiter_key) = std::env::var("JUPITER_API_KEY") {
        cmd.env("JUPITER_API_KEY", jupiter_key);
    }

    Ok(cmd)
}

fn read_local_key(filename: &str) -> Result<String> {
    let path = workspace_path("tests/src/common/local-keys").join(filename);
    Ok(fs::read_to_string(&path)
        .with_context(|| format!("failed to read local key {}", path.display()))?
        .trim()
        .to_string())
}

fn spawn_kora(config: &Path, signers: &Path, rpc_url: &str, port: u16) -> Result<Child> {
    let verbose = std::env::var("KORA_TEST_VERBOSE").is_ok();
    let (stdout, stderr) =
        if verbose { (Stdio::inherit(), Stdio::inherit()) } else { (Stdio::null(), Stdio::null()) };

    let child = kora_command(config, rpc_url)?
        .args(["rpc", "start", "--signers-config"])
        .arg(signers)
        .args(["--port", &port.to_string()])
        .stdout(stdout)
        .stderr(stderr)
        .kill_on_drop(true)
        .spawn()?;
    Ok(child)
}

async fn initialize_payment_atas(config: &Path, signers: &Path, rpc_url: &str) -> Result<()> {
    let output = kora_command(config, rpc_url)?
        .args(["rpc", "initialize-atas", "--signers-config"])
        .arg(signers)
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to initialize payment ATAs: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

async fn wait_for_liveness(kora: &mut Child, port: u16) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/liveness");
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut delay = Duration::from_millis(50);
    let mut last_status = None;

    while Instant::now() < deadline {
        if let Some(status) = kora.try_wait()? {
            return Err(anyhow!(
                "Kora exited with {status} before becoming ready. \
                Set KORA_TEST_VERBOSE=1 to see its output."
            ));
        }
        if let Ok(response) = client.get(&url).timeout(Duration::from_secs(5)).send().await {
            let status = response.status();
            if status.is_success() {
                return Ok(());
            }
            // Fixtures that set `liveness = false` answer 500, so a failing
            // status still means Kora, as long as the body is its JSON-RPC error.
            if response.json::<serde_json::Value>().await.is_ok_and(|b| b.get("jsonrpc").is_some())
            {
                return Ok(());
            }
            last_status = Some(status);
        }
        tokio::time::sleep(delay).await;
        delay = std::cmp::min(delay * 2, Duration::from_secs(1));
    }

    // The port was free when we picked it, not when Kora bound it, so something
    // else can be holding it by now.
    match last_status {
        Some(status) => Err(anyhow!(
            "port {port} answered /liveness with {status} and no JSON-RPC body, \
            so it is not our Kora"
        )),
        None => Err(anyhow!("Kora server on port {port} did not become reachable")),
    }
}

/// A `static OnceCell` harness is never dropped, so `kill_on_drop` never fires
/// and the child Kora would outlive the test binary.
fn register_kora_teardown(child: &Child) -> Result<()> {
    let pid = child.id().ok_or_else(|| anyhow!("Kora child exited before it could be tracked"))?;
    KORA_PID.store(pid as i32, Ordering::SeqCst);
    if unsafe { libc::atexit(stop_kora_at_exit) } != 0 {
        return Err(anyhow!("failed to register Kora teardown hook"));
    }
    Ok(())
}

/// SIGTERM first so an llvm-cov-instrumented Kora can flush coverage data;
/// SIGKILL only if it does not terminate in time.
extern "C" fn stop_kora_at_exit() {
    let pid = KORA_PID.swap(0, Ordering::SeqCst);
    if pid <= 0 {
        return;
    }
    unsafe { libc::kill(pid, libc::SIGTERM) };

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let mut status = 0;
        if unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) } != 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    unsafe { libc::kill(pid, libc::SIGKILL) };
}

#[ctor::dtor]
fn remove_rendered_config() {
    if let Some(path) = RENDERED_CONFIG.get() {
        let _ = fs::remove_file(path);
    }
}
