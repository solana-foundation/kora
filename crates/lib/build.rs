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
    
    match config {
        toml::Value::Table(table) => {
            for (key, value) in table {
                output.push_str(&generate_value(key, value));
            }
        }
        _ => panic!("Root must be a TOML table"),
    }
    
    output
}

fn generate_value(key: &str, value: &toml::Value) -> String {
    match value {
        toml::Value::Array(arr) if key == "allowed_instructions" => {
            let mut structs = String::new();
            
            // Generate the struct definition
            structs.push_str("#[derive(Debug)]\n");
            structs.push_str("pub struct AllowedInstruction {\n");
            structs.push_str("    pub program: &'static str,\n");
            structs.push_str("    pub instructions: &'static [&'static str],\n");
            structs.push_str("}\n\n");

            // Generate the constant array
            structs.push_str("pub const ALLOWED_INSTRUCTIONS: &[AllowedInstruction] = &[\n");
            
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
                    
                    structs.push_str("    AllowedInstruction {\n");
                    structs.push_str(&format!("        program: \"{}\",\n", program));
                    structs.push_str(&format!("        instructions: &[{}],\n", 
                        instructions.iter()
                            .map(|i| format!("\"{}\"", i))
                            .collect::<Vec<_>>()
                            .join(", ")));
                    structs.push_str("    },\n");
                }
            }
            
            structs.push_str("];\n");
            structs
        }
        toml::Value::String(s) => format!("pub const {}: &str = \"{}\";\n", key.to_uppercase(), s),
        toml::Value::Integer(i) => format!("pub const {}: i64 = {};\n", key.to_uppercase(), i),
        toml::Value::Float(f) => format!("pub const {}: f64 = {};\n", key.to_uppercase(), f),
        toml::Value::Boolean(b) => format!("pub const {}: bool = {};\n", key.to_uppercase(), b),
        toml::Value::Table(table) => {
            let mut nested = String::new();
            nested.push_str(&format!("pub mod {} {{\n", key.to_lowercase()));
            for (k, v) in table {
                nested.push_str(&generate_value(k, v));
            }
            nested.push_str("}\n");
            nested
        }
        _ => String::new(),
    }
}