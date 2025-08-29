use std::env;

use crate::KoraError;

pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, anyhow::Error> {
    if !hex.len().is_multiple_of(2) {
        return Err(anyhow::anyhow!("Hex string must have even length"));
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub fn bytes_to_hex(bytes: &[u8]) -> Result<String, anyhow::Error> {
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn get_env_var_for_signer(env_var_name: &str, signer_name: &str) -> Result<String, KoraError> {
    env::var(env_var_name).map_err(|_| {
        KoraError::ValidationError(format!(
            "Environment variable '{env_var_name}' required for signer '{signer_name}' is not set"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;

    #[test]
    fn test_hex_to_bytes_valid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = hex_to_bytes("48656c6c6f").unwrap();
        assert_eq!(result, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn test_hex_to_bytes_empty() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = hex_to_bytes("").unwrap();
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_hex_to_bytes_invalid_length() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = hex_to_bytes("48656c6c6");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_to_bytes_invalid_chars() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = hex_to_bytes("zzaa");
        assert!(result.is_err());
    }

    #[test]
    fn test_bytes_to_hex_valid() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = bytes_to_hex(&[0x48, 0x65, 0x6c, 0x6c, 0x6f]).unwrap();
        assert_eq!(result, "48656c6c6f");
    }

    #[test]
    fn test_bytes_to_hex_empty() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = bytes_to_hex(&[]).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_bytes_to_hex_single_byte() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let result = bytes_to_hex(&[0xff]).unwrap();
        assert_eq!(result, "ff");
    }

    #[test]
    fn test_get_env_var_for_signer_exists() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        env::set_var("TEST_SIGNER_VAR", "test_value");
        let result = get_env_var_for_signer("TEST_SIGNER_VAR", "test_signer").unwrap();
        assert_eq!(result, "test_value");
        env::remove_var("TEST_SIGNER_VAR");
    }

    #[test]
    fn test_get_env_var_for_signer_not_exists() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        env::remove_var("NONEXISTENT_VAR");
        let result = get_env_var_for_signer("NONEXISTENT_VAR", "test_signer");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_roundtrip_hex_conversion() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let original = vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
        let hex = bytes_to_hex(&original).unwrap();
        let converted = hex_to_bytes(&hex).unwrap();
        assert_eq!(original, converted);
    }
}
