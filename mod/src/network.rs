//! Mod-owned private protocol utilities, independent of Horizon's engine crates.
pub mod packets {
    pub mod configuration {
        pub struct CustomPayload {
            pub channel: String,
            pub data: Vec<u8>,
        }
    }
    pub mod play {
        pub enum Serverbound {
            CustomPayload(super::configuration::CustomPayload),
        }
    }
}
pub mod crypto {
    pub fn md5(bytes: &[u8]) -> [u8; 16] {
        md5::compute(bytes).0
    }
}
pub mod rsa {
    use rsa::{Pkcs1v15Encrypt, pkcs8::EncodePublicKey};
    pub struct RsaPrivateKey(rsa::PrivateKey);
    impl RsaPrivateKey {
        pub fn generate(bits: usize) -> Result<Self, String> {
            rsa::PrivateKey::new(&mut rand::rngs::SysRng, bits)
                .map(Self)
                .map_err(|e| e.to_string())
        }
        pub fn public_der(&self) -> Vec<u8> {
            self.0
                .to_public_key()
                .to_public_key_der()
                .expect("valid RSA public key")
                .as_bytes()
                .to_vec()
        }
        pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
            self.0
                .decrypt_blinded(&mut rand::rngs::SysRng, Pkcs1v15Encrypt, data)
                .map_err(|e| e.to_string())
        }
    }
}
pub mod servers {
    pub fn parse_address(address: &str) -> Option<(String, u16)> {
        if let Some(rest) = address.strip_prefix('[') {
            let (host, tail) = rest.split_once(']')?;
            return Some((
                host.into(),
                tail.strip_prefix(':')
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(25565),
            ));
        }
        if let Some((host, port)) = address.rsplit_once(':')
            && !host.contains(':')
        {
            return Some((host.into(), port.parse().ok()?));
        }
        (!address.is_empty()).then(|| (address.into(), 25565))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires the OpenSSL command; checks the server's independent RSA format"]
    fn openssl_pkcs1v15_key_exchange() {
        use std::{fs, process::Command};
        let root = std::env::temp_dir().join(format!("plasmo-rsa-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let key = super::rsa::RsaPrivateKey::generate(2048).unwrap();
        fs::write(root.join("public.der"), key.public_der()).unwrap();
        let secret = b"voice-session-key";
        fs::write(root.join("plain"), secret).unwrap();
        let result = Command::new("openssl")
            .current_dir(&root)
            .args([
                "pkeyutl",
                "-encrypt",
                "-pubin",
                "-keyform",
                "DER",
                "-inkey",
                "public.der",
                "-in",
                "plain",
                "-out",
                "cipher",
                "-pkeyopt",
                "rsa_padding_mode:pkcs1",
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let cipher = fs::read(root.join("cipher")).unwrap();
        assert_eq!(cipher.len(), 256);
        assert_eq!(key.decrypt(&cipher).unwrap(), secret);
        assert!(key.decrypt(&cipher[..255]).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
