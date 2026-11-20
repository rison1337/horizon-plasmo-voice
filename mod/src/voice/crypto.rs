// Protocol framing adapted from mineflayer-plasmovoice (MIT).
// Copyright (c) 2023 Maksim Klimenko. See THIRD_PARTY_NOTICES.md.
use crate::Result;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};

/// Only audio bodies are encrypted. Each body has its own random 16-byte IV.
pub struct AudioCipher([u8; 16]);
impl AudioCipher {
    pub fn new(key: &[u8]) -> Result<Self> {
        Ok(Self(key.try_into().map_err(|_| "expected an AES-128 key")?))
    }
    pub fn encrypt(&self, opus: &[u8]) -> Result<Vec<u8>> {
        let mut iv = [0; 16];
        getrandom::fill(&mut iv).map_err(|e| e.to_string())?;
        self.encrypt_with_iv(opus, iv)
    }
    pub fn encrypt_with_iv(&self, opus: &[u8], iv: [u8; 16]) -> Result<Vec<u8>> {
        if opus.len() > 4096 {
            return Err("Opus packet too large".into());
        }
        let mut out = vec![0; 16 + (opus.len() / 16 + 1) * 16];
        out[..16].copy_from_slice(&iv);
        out[16..16 + opus.len()].copy_from_slice(opus);
        cbc::Encryptor::<aes::Aes128>::new(&self.0.into(), &iv.into())
            .encrypt_padded_mut::<Pkcs7>(&mut out[16..], opus.len())
            .map_err(|_| "AES padding failed")?;
        Ok(out)
    }
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        if encrypted.len() < 32 || encrypted.len() > 4144 || !encrypted.len().is_multiple_of(16) {
            return Err("invalid encrypted audio length".into());
        }
        let iv: [u8; 16] = encrypted[..16].try_into().unwrap();
        let mut bytes = encrypted[16..].to_vec();
        let len = cbc::Decryptor::<aes::Aes128>::new(&self.0.into(), &iv.into())
            .decrypt_padded_mut::<Pkcs7>(&mut bytes)
            .map_err(|_| "invalid encrypted audio padding")?
            .len();
        bytes.truncate(len);
        Ok(bytes)
    }
}
impl Drop for AudioCipher {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}
