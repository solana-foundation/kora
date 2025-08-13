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

    // Update prometheus.yml
    update_prometheus_yml(&metrics_dir, metrics.port, &metrics.endpoint, metrics.scrape_interval)?;

    // Update docker-compose.metrics.yml
    update_docker_compose(&metrics_dir, metrics.port, &metrics.endpoint)?;

    println!("✅ Configuration files updated successfully!");
    println!();
    println!("To start the metrics stack, run:");
    println!("  docker compose -f docker-compose.metrics.yml up -d");

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

fn update_prometheus_yml(
    metrics_dir: &Path,
    port: u16,
    endpoint: &str,
    scrape_interval: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = metrics_dir.join("prometheus.yml");

    if file_path.exists() {
        println!("Updating prometheus.yml (preserving custom modifications)...");

        let content = fs::read_to_string(&file_path)?;
        let mut updated_content = content;

        // Update scrape intervals
        updated_content = regex::Regex::new(r"scrape_interval: \d+s")?
            .replace_all(&updated_content, &format!("scrape_interval: {scrape_interval}s"))
            .to_string();

        // Update evaluation interval
        updated_content = regex::Regex::new(r"evaluation_interval: \d+s")?
            .replace_all(&updated_content, &format!("evaluation_interval: {scrape_interval}s"))
            .to_string();

        // Update kora target port - use host.docker.internal for Docker containers to access host
        updated_content = regex::Regex::new(r#""(kora|host\.docker\.internal):\d+""#)?
            .replace_all(&updated_content, &format!("\"host.docker.internal:{port}\""))
            .to_string();

        // Update metrics_path
        updated_content = regex::Regex::new(r#"metrics_path: "[^"]*""#)?
            .replace_all(&updated_content, &format!("metrics_path: \"{endpoint}\""))
            .to_string();

        fs::write(&file_path, updated_content)?;
        println!("  ✓ Updated port: {port}");
        println!("  ✓ Updated endpoint: {endpoint}");
        println!("  ✓ Updated scrape interval: {scrape_interval}s");
    } else {
        println!("⚠ prometheus.yml not found, creating default...");
        let default_config = format!(
            r#"global:
  scrape_interval: {scrape_interval}s
  evaluation_interval: {scrape_interval}s

scrape_configs:
  - job_name: "prometheus"
    static_configs:
      - targets: ["localhost:9090"]

  - job_name: "kora"
    static_configs:
      - targets: ["host.docker.internal:{port}"]
    metrics_path: "{endpoint}"
    scrape_interval: {scrape_interval}s
    scrape_timeout: {scrape_interval}s
"#
        );
        fs::write(&file_path, default_config)?;
    }

    Ok(())
}

fn update_docker_compose(
    metrics_dir: &Path,
    _port: u16,
    _endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = metrics_dir.join("docker-compose.metrics.yml");

    if file_path.exists() {
        println!("docker-compose.metrics.yml exists (no updates needed)...");
    } else {
        println!("⚠ docker-compose.metrics.yml not found, creating default...");
        let default_config = r#"services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    restart: unless-stopped
    networks:
      - metrics

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    env_file:
      - ../../.env
    volumes:
      - grafana-storage:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
    restart: unless-stopped
    networks:
      - metrics

networks:
  metrics:
    driver: bridge

volumes:
  grafana-storage:
"#;
        fs::write(&file_path, default_config)?;
    }

    Ok(())
}
