use std::fs;
use std::path::Path;
use toml;

fn main() {
    // Create generated directory if it doesn't exist
    let generated_dir = Path::new("src/generated");
    fs::create_dir_all(generated_dir).unwrap();

    let config_str = fs::read_to_string("kora.toml").unwrap();
    let config: toml::Value = toml::from_str(&config_str).unwrap();
    
    // Generate Rust code with all TOML values as constants
    let generated_code = generate_constants(&config);
    
    // Write to the generated file
    fs::write(
        generated_dir.join("config_generated.rs"),
        generated_code
    ).unwrap();
}

fn generate_constants(config: &toml::Value) -> String {
    let mut output = String::new();
    
    if let Some(validation_config) = config.get("validation_config")
        .and_then(|v| v.as_table()) {
        // Generate struct for allowed instructions
        output.push_str("#[derive(Debug)]\n");
        output.push_str("pub struct AllowedInstruction {\n");
        output.push_str("    pub program: &'static str,\n");
        output.push_str("    pub instructions: &'static [&'static str],\n");
        output.push_str("}\n\n");

        // Generate all constants
        for (key, value) in validation_config {
            match (key.as_str(), value) {
                ("max_allowed_lamports", toml::Value::Integer(n)) => {
                    output.push_str(&format!("pub const MAX_ALLOWED_LAMPORTS: u64 = {};\n", n));
                }
                ("max_signatures", toml::Value::Integer(n)) => {
                    output.push_str(&format!("pub const MAX_SIGNATURES: u32 = {};\n", n));
                }
                ("allowed_programs", toml::Value::Array(arr)) => {
                    output.push_str(&format!("pub const ALLOWED_PROGRAMS: &[&str] = &[{}];\n",
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", ")));
                }
                ("allowed_tokens", toml::Value::Array(arr)) => {
                    output.push_str(&format!("pub const ALLOWED_TOKENS: &[&str] = &[{}];\n",
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", ")));
                }
                ("allowed_spl_paid_tokens", toml::Value::Array(arr)) => {
                    output.push_str(&format!("pub const ALLOWED_SPL_PAID_TOKENS: &[&str] = &[{}];\n",
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", ")));
                }
                ("disallowed_accounts", toml::Value::Array(arr)) => {
                    output.push_str(&format!("pub const DISALLOWED_ACCOUNTS: &[&str] = &[{}];\n",
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", ")));
                }
                ("allowed_instructions", toml::Value::Array(arr)) => {
                    output.push_str("pub const ALLOWED_INSTRUCTIONS: &[AllowedInstruction] = &[\n");
                    for item in arr {
                        if let toml::Value::Table(table) = item {
                            let program = table.get("program")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default();
                            
                            let instructions = table.get("instructions")
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>())
                                .unwrap_or_default();
                            
                            output.push_str("    AllowedInstruction {\n");
                            output.push_str(&format!("        program: \"{}\",\n", program));
                            output.push_str(&format!("        instructions: &[{}],\n", 
                                instructions.iter()
                                    .map(|i| format!("\"{}\"", i))
                                    .collect::<Vec<_>>()
                                    .join(", ")));
                            output.push_str("    },\n");
                        }
                    }
                    output.push_str("];\n");
                }
                _ => {}
            }
        }
    }
    
    output
}