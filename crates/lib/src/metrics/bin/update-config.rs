use std::{
    env, fs,
    path::{Path, PathBuf},
};

use kora_lib::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the current working directory
    let current_dir = env::current_dir()?;

    // Find the project root by looking for kora.toml
    let project_root = find_project_root(&current_dir)?;
    let config_path = project_root.join("kora.toml");

    if !config_path.exists() {
        eprintln!("Error: kora.toml not found at {config_path:?}");
        eprintln!("Please ensure you're in a Kora project directory");
        std::process::exit(1);
    }

    // Load config
    let config = Config::load_config(&config_path)?;
    let metrics = &config.metrics;

    println!("Reading configuration from kora.toml:");
    println!("  Enabled: {}", metrics.enabled);
    println!("  Endpoint: {}", metrics.endpoint);
    println!("  Port: {}", metrics.port);
    println!("  Scrape Interval: {}s", metrics.scrape_interval);
    println!();

    // Metrics directory is always relative to project root
    let metrics_dir = project_root.join("crates/lib/src/metrics");

    let mut updated_files = 0;

    // Update prometheus.yml
    if update_prometheus_yml(
        &metrics_dir,
        metrics.port,
        &metrics.endpoint,
        metrics.scrape_interval,
    )? {
        updated_files += 1;
    }

    // Update docker-compose.metrics.yml (Grafana port only)
    if update_docker_compose(&metrics_dir)? {
        updated_files += 1;
    }

    // Update Grafana datasources.yml (Prometheus port)
    if update_grafana_datasources(&metrics_dir)? {
        updated_files += 1;
    }

    if updated_files > 0 {
        println!("✅ Updated {updated_files} configuration file(s)");
    } else {
        println!("ℹ️  All configuration files are already up to date");
    }

    Ok(())
}

/// Find the project root by traversing up until we find kora.toml or Cargo.toml with workspace
fn find_project_root(start_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current = start_dir.to_path_buf();

    loop {
        // Check if kora.toml exists in current directory
        if current.join("kora.toml").exists() {
            return Ok(current);
        }

        // Check if this is the workspace root (has Cargo.toml with [workspace])
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        // Move up one directory
        if !current.pop() {
            return Err(
                "Could not find project root (no kora.toml or workspace Cargo.toml found)".into()
            );
        }
    }
}

/// Update prometheus.yml with Kora server configuration
fn update_prometheus_yml(
    metrics_dir: &Path,
    port: u16,
    endpoint: &str,
    scrape_interval: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let file_path = metrics_dir.join("prometheus.yml");

    if !file_path.exists() {
        println!("⚠️  prometheus.yml not found at {}", file_path.display());
        return Ok(false);
    }

    println!("Updating prometheus.yml...");
    let content = fs::read_to_string(&file_path)?;
    let mut updated_content = content.clone();
    let mut changes_made = false;

    // Update global scrape intervals
    if let Ok(regex) = regex::Regex::new(r"scrape_interval:\s*\d+s") {
        let new_content =
            regex.replace_all(&updated_content, &format!("scrape_interval: {scrape_interval}s"));
        if new_content != updated_content {
            updated_content = new_content.to_string();
            changes_made = true;
        }
    }

    // Update evaluation interval
    if let Ok(regex) = regex::Regex::new(r"evaluation_interval:\s*\d+s") {
        let new_content = regex
            .replace_all(&updated_content, &format!("evaluation_interval: {scrape_interval}s"));
        if new_content != updated_content {
            updated_content = new_content.to_string();
            changes_made = true;
        }
    }

    // Update kora target port - use host.docker.internal for Docker containers to access host
    if let Ok(regex) = regex::Regex::new(r#""(kora|host\.docker\.internal):\d+""#) {
        let new_content =
            regex.replace_all(&updated_content, &format!("\"host.docker.internal:{port}\""));
        if new_content != updated_content {
            updated_content = new_content.to_string();
            changes_made = true;
        }
    }

    // Update metrics_path for kora job
    if let Ok(regex) = regex::Regex::new(r#"metrics_path:\s*"[^"]*""#) {
        let new_content =
            regex.replace_all(&updated_content, &format!("metrics_path: \"{endpoint}\""));
        if new_content != updated_content {
            updated_content = new_content.to_string();
            changes_made = true;
        }
    }

    if changes_made {
        fs::write(&file_path, updated_content)?;
        println!("  ✓ Updated Kora target: host.docker.internal:{port}");
        println!("  ✓ Updated endpoint: {endpoint}");
        println!("  ✓ Updated intervals: {scrape_interval}s");
    } else {
        println!("  ℹ️  Already up to date");
    }

    Ok(changes_made)
}

/// Update docker-compose.metrics.yml (currently no dynamic updates needed)
fn update_docker_compose(metrics_dir: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let file_path = metrics_dir.join("docker-compose.metrics.yml");

    if !file_path.exists() {
        println!("⚠️  docker-compose.metrics.yml not found at {}", file_path.display());
        return Ok(false);
    }

    // For now, docker-compose doesn't need dynamic updates
    // Ports are hardcoded in the compose file (9090 for Prometheus, 3000 for Grafana)
    println!("docker-compose.metrics.yml exists (no updates needed)");
    Ok(false)
}

/// Update Grafana datasources.yml to ensure correct Prometheus URL
fn update_grafana_datasources(metrics_dir: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let file_path = metrics_dir.join("grafana/provisioning/datasources/datasources.yml");

    if !file_path.exists() {
        println!("⚠️  Grafana datasources.yml not found at {}", file_path.display());
        return Ok(false);
    }

    println!("Updating Grafana datasources.yml...");
    let content = fs::read_to_string(&file_path)?;
    let mut updated_content = content.clone();
    let mut changes_made = false;

    // Ensure Prometheus URL uses the correct container name and port
    if let Ok(regex) = regex::Regex::new(r"url:\s*http://[^:\s]+:9090") {
        let new_content = regex.replace_all(&updated_content, "url: http://prometheus:9090");
        if new_content != updated_content {
            updated_content = new_content.to_string();
            changes_made = true;
        }
    }

    if changes_made {
        fs::write(&file_path, updated_content)?;
        println!("  ✓ Updated Prometheus URL to http://prometheus:9090");
    } else {
        println!("  ℹ️  Already up to date");
    }

    Ok(changes_made)
}
