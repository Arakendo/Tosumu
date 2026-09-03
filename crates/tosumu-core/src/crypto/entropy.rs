use crate::error::{Result, TosumuError};

/// Private system-entropy facade for the format-v3 construction.
///
/// Tosumu owns each purpose and byte length. This facade deliberately exposes
/// no runtime selection or public injection point.
pub(crate) struct SystemEntropy;

impl SystemEntropy {
    pub(crate) fn dek() -> Result<[u8; 32]> {
        Self::fallible_bytes()
    }

    pub(crate) fn nonce() -> Result<[u8; 12]> {
        Self::fallible_bytes()
    }

    pub(crate) fn passphrase_salt() -> Result<[u8; 16]> {
        Self::fallible_bytes()
    }

    pub(crate) fn database_identifier_seed() -> Result<[u8; 8]> {
        Self::fallible_bytes()
    }

    /// Preserve the existing infallible public recovery-secret API and panic
    /// behavior. Changing it requires separate error-contract work.
    pub(crate) fn recovery_secret_bytes() -> [u8; 20] {
        let mut bytes = [0u8; 20];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        bytes
    }

    fn fallible_bytes<const N: usize>() -> Result<[u8; N]> {
        let mut bytes = [0u8; N];
        getrandom::getrandom(&mut bytes).map_err(|_| TosumuError::RngFailed)?;
        Ok(bytes)
    }
}
