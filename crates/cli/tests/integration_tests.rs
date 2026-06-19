use std::{fs::File, io::Write, process::Command};

#[test]
fn test_config_validate_invalid_config_file() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join(format!("test_invalid_config_{}.toml", std::process::id()));

    // Load default config and inject invalid pubkey to trigger validation error
    let base_config_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kora.toml");

    let base_config = std::fs::read_to_string(&base_config_path)
        .expect("kora.toml must exist at repo root to run this test");

    let config_content = base_config.replace(
        "allowed_programs = [",
        "allowed_programs = [\n    \"invalid_pubkey_format_that_will_fail\",",
    );

    assert!(
        config_content.contains("invalid_pubkey_format_that_will_fail"),
        "Injection guard failed: The string 'allowed_programs = [' was not found in kora.toml."
    );

    let mut file = File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kora"))
        .arg("--config")
        .arg(&config_path)
        .arg("config")
        .arg("validate")
        .output()
        .unwrap();

    let _ = std::fs::remove_file(&config_path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration validation failed"));
    assert!(!output.status.success());
}
