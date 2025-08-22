use solana_message::VersionedMessage;

use crate::error::KoraError;
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub trait VersionedMessageExt {
    fn encode_b64_message(&self) -> Result<String, KoraError>;
}

impl VersionedMessageExt for VersionedMessage {
    fn encode_b64_message(&self) -> Result<String, KoraError> {
        let serialized = self.serialize();
        Ok(STANDARD.encode(serialized))
    }
}
