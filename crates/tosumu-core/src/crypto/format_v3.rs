use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::entropy::SystemEntropy;
use super::{make_aad, read_kdf_u32, wrap_aad, KCV_AAD, KCV_KNOWN_PT, KCV_NONCE};
use crate::error::{Result, TosumuError};
use crate::format::{
    CIPHERTEXT_OFFSET, FILE_HEADER_PLAIN_LEN, KEYSLOT_REGION_OFFSET, KEYSLOT_SIZE, NONCE_SIZE,
    PAGE_FRAME_TYPE_OFFSET, PAGE_PLAINTEXT_SIZE, PAGE_SIZE, PAGE_VERSION_OFFSET, PAGE_VERSION_SIZE,
    TAG_SIZE,
};

type HmacSha256 = Hmac<Sha256>;

/// The one cryptographic construction defined by format v3.
///
/// This is a private concrete facade, not a stable provider interface.
pub(super) struct FormatV3Crypto;

impl FormatV3Crypto {
    pub(super) fn derive_subkeys(dek: &[u8; 32]) -> ([u8; 32], [u8; 32], [u8; 32]) {
        let hk = Hkdf::<Sha256>::new(None, dek);
        let mut page_key = [0u8; 32];
        let mut header_mac_key = [0u8; 32];
        let mut audit_key = [0u8; 32];
        hk.expand(b"tosumu/v1/page", &mut page_key)
            .expect("HKDF expand: output length is valid");
        hk.expand(b"tosumu/v1/header-mac", &mut header_mac_key)
            .expect("HKDF expand: output length is valid");
        hk.expand(b"tosumu/v1/audit", &mut audit_key)
            .expect("HKDF expand: output length is valid");
        (page_key, header_mac_key, audit_key)
    }

    pub(super) fn encrypt_page(
        page_key: &[u8; 32],
        pgno: u64,
        page_version: u64,
        page_type: u8,
        plaintext: &[u8; PAGE_PLAINTEXT_SIZE],
    ) -> Result<[u8; PAGE_SIZE]> {
        let nonce = SystemEntropy::nonce()?;
        let aad = make_aad(pgno, page_version, page_type);
        let cipher = ChaCha20Poly1305::new(page_key.into());
        let ciphertext = cipher
            .encrypt(
                nonce.as_slice().into(),
                Payload {
                    msg: plaintext.as_slice(),
                    aad: &aad,
                },
            )
            .map_err(|_| TosumuError::EncryptFailed)?;
        if ciphertext.len() != PAGE_PLAINTEXT_SIZE + TAG_SIZE {
            return Err(TosumuError::EncryptFailed);
        }
        let mut frame = [0u8; PAGE_SIZE];
        frame[..NONCE_SIZE].copy_from_slice(&nonce);
        frame[PAGE_VERSION_OFFSET..PAGE_VERSION_OFFSET + PAGE_VERSION_SIZE]
            .copy_from_slice(&page_version.to_le_bytes());
        frame[PAGE_FRAME_TYPE_OFFSET] = page_type;
        frame[CIPHERTEXT_OFFSET..].copy_from_slice(&ciphertext);
        Ok(frame)
    }

    pub(super) fn decrypt_page(
        page_key: &[u8; 32],
        pgno: u64,
        frame: &[u8; PAGE_SIZE],
    ) -> Result<([u8; PAGE_PLAINTEXT_SIZE], u64)> {
        let nonce: [u8; NONCE_SIZE] =
            frame[..NONCE_SIZE]
                .try_into()
                .map_err(|_| TosumuError::Corrupt {
                    pgno,
                    reason: "bad nonce length",
                })?;
        let page_version = u64::from_le_bytes(
            frame[PAGE_VERSION_OFFSET..PAGE_VERSION_OFFSET + PAGE_VERSION_SIZE]
                .try_into()
                .map_err(|_| TosumuError::Corrupt {
                    pgno,
                    reason: "bad page_version length",
                })?,
        );
        let page_type = frame[PAGE_FRAME_TYPE_OFFSET];
        let aad = make_aad(pgno, page_version, page_type);
        let cipher = ChaCha20Poly1305::new(page_key.into());
        let plaintext = cipher
            .decrypt(
                nonce.as_slice().into(),
                Payload {
                    msg: &frame[CIPHERTEXT_OFFSET..],
                    aad: &aad,
                },
            )
            .map_err(|_| TosumuError::AuthFailed { pgno: Some(pgno) })?;
        if plaintext.len() != PAGE_PLAINTEXT_SIZE {
            return Err(TosumuError::Corrupt {
                pgno,
                reason: "decrypted page has wrong length",
            });
        }
        let mut out = [0u8; PAGE_PLAINTEXT_SIZE];
        out.copy_from_slice(&plaintext);
        Ok((out, page_version))
    }

    pub(super) fn derive_passphrase_kek(
        passphrase: &str,
        salt: &[u8; 16],
        kdf_params: &[u8; 32],
    ) -> Result<[u8; 32]> {
        let (m, t, p) = if kdf_params[..16].iter().all(|&byte| byte == 0) {
            (
                super::ARGON2_M_COST,
                super::ARGON2_T_COST,
                super::ARGON2_P_COST,
            )
        } else {
            (
                read_kdf_u32(kdf_params, 0),
                read_kdf_u32(kdf_params, 4),
                read_kdf_u32(kdf_params, 8),
            )
        };
        let params = Params::new(m, t, p, Some(32))
            .map_err(|_| TosumuError::InvalidArgument("invalid Argon2id parameters"))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut kek = [0u8; 32];
        argon2
            .hash_password_into(passphrase.as_bytes(), salt, &mut kek)
            .map_err(|_| TosumuError::InvalidArgument("Argon2id hashing failed"))?;
        Ok(kek)
    }

    pub(super) fn wrap_dek(
        kek: &[u8; 32],
        dek: &[u8; 32],
        slot_index: u16,
        dek_id: u64,
        kind: u8,
    ) -> Result<([u8; 12], [u8; 48])> {
        let nonce = SystemEntropy::nonce()?;
        let cipher = ChaCha20Poly1305::new(kek.into());
        let ciphertext = cipher
            .encrypt(
                nonce.as_slice().into(),
                Payload {
                    msg: dek.as_slice(),
                    aad: &wrap_aad(slot_index, dek_id, kind),
                },
            )
            .map_err(|_| TosumuError::EncryptFailed)?;
        debug_assert_eq!(ciphertext.len(), 48);
        let mut wrapped = [0u8; 48];
        wrapped.copy_from_slice(&ciphertext);
        Ok((nonce, wrapped))
    }

    pub(super) fn unwrap_dek(
        kek: &[u8; 32],
        nonce: &[u8; 12],
        wrapped: &[u8; 48],
        slot_index: u16,
        dek_id: u64,
        kind: u8,
    ) -> Result<[u8; 32]> {
        let cipher = ChaCha20Poly1305::new(kek.into());
        let plaintext = cipher
            .decrypt(
                nonce.as_slice().into(),
                Payload {
                    msg: wrapped.as_slice(),
                    aad: &wrap_aad(slot_index, dek_id, kind),
                },
            )
            .map_err(|_| TosumuError::WrongKey)?;
        debug_assert_eq!(plaintext.len(), 32);
        let mut dek = [0u8; 32];
        dek.copy_from_slice(&plaintext);
        Ok(dek)
    }

    pub(super) fn compute_kcv(kek: &[u8; 32]) -> [u8; 32] {
        let cipher = ChaCha20Poly1305::new(kek.into());
        let ciphertext = cipher
            .encrypt(
                KCV_NONCE.as_slice().into(),
                Payload {
                    msg: &KCV_KNOWN_PT,
                    aad: KCV_AAD,
                },
            )
            .expect("KCV encryption: ChaCha20-Poly1305 over fixed inputs cannot fail");
        debug_assert_eq!(ciphertext.len(), 32);
        let mut kcv = [0u8; 32];
        kcv.copy_from_slice(&ciphertext);
        kcv
    }

    pub(super) fn verify_kcv(kek: &[u8; 32], kcv: &[u8; 32]) -> Result<()> {
        ChaCha20Poly1305::new(kek.into())
            .decrypt(
                KCV_NONCE.as_slice().into(),
                Payload {
                    msg: kcv.as_slice(),
                    aad: KCV_AAD,
                },
            )
            .map_err(|_| TosumuError::WrongKey)?;
        Ok(())
    }

    pub(super) fn compute_header_mac(
        header_mac_key: &[u8; 32],
        page0: &[u8; PAGE_SIZE],
        keyslot_count: usize,
    ) -> [u8; 32] {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(header_mac_key)
            .expect("HMAC: key length is always valid for SHA-256");
        mac.update(&page0[..FILE_HEADER_PLAIN_LEN]);
        let keyslot_end = KEYSLOT_REGION_OFFSET + keyslot_count * KEYSLOT_SIZE;
        mac.update(&page0[KEYSLOT_REGION_OFFSET..keyslot_end]);
        let mut output = [0u8; 32];
        output.copy_from_slice(&mac.finalize().into_bytes());
        output
    }

    pub(super) fn verify_header_mac(
        header_mac_key: &[u8; 32],
        page0: &[u8; PAGE_SIZE],
        keyslot_count: usize,
        expected_mac: &[u8; 32],
    ) -> Result<()> {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(header_mac_key)
            .expect("HMAC: key length is always valid for SHA-256");
        mac.update(&page0[..FILE_HEADER_PLAIN_LEN]);
        let keyslot_end = KEYSLOT_REGION_OFFSET + keyslot_count * KEYSLOT_SIZE;
        mac.update(&page0[KEYSLOT_REGION_OFFSET..keyslot_end]);
        mac.verify_slice(expected_mac)
            .map_err(|_| TosumuError::AuthFailed { pgno: None })
    }

    pub(super) fn derive_recovery_kek(raw: &[u8]) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, raw);
        let mut kek = [0u8; 32];
        hk.expand(b"tosumu/v1/recovery-kek", &mut kek)
            .expect("HKDF expand: output length is valid");
        kek
    }
}
