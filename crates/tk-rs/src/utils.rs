pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, anyhow::Error> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub fn bytes_to_hex(bytes: &[u8]) -> Result<String, anyhow::Error> {
    use std::fmt::Write;
    Ok(bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut acc, byte| {
        let _ = write!(acc, "{:02x}", byte);
        acc
    }))
}
