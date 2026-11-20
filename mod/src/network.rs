//! Mod-owned private protocol utilities, independent of Horizon's engine crates.

/// A vanilla plugin-channel body. This deliberately uses the SDK's generic play
/// packet API: Plasmo/Paper does not negotiate Horizon's registered transport.
pub fn plugin_packet(channel: &str, data: &[u8]) -> Result<mod_api::ServerboundPlayPacket, String> {
    if data.len() > 32767 {
        return Err("plugin payload exceeds the serverbound limit".into());
    }
    mod_api::Identifier::parse(channel).map_err(|e| format!("{e:?}"))?;
    let mut body = Vec::with_capacity(channel.len() + data.len() + 5);
    let mut length = channel.len();
    loop {
        let byte = (length & 127) as u8;
        length >>= 7;
        body.push(byte | if length != 0 { 128 } else { 0 });
        if length == 0 {
            break;
        }
    }
    body.extend_from_slice(channel.as_bytes());
    body.extend_from_slice(data);
    Ok(mod_api::ServerboundPlayPacket::new(
        mod_api::Identifier::parse("minecraft:custom_payload").unwrap(),
        body,
    ))
}
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
    fn paper_channel_announcement_contains_all_voice_capabilities() {
        let packet = super::plugin_packet("minecraft:register", crate::protocol::REGISTER).unwrap();
        assert_eq!(packet.id.to_string(), "minecraft:custom_payload");
        assert_eq!(packet.body,
            b"\x12minecraft:registerplasmo:voice/v2/installed\0plasmo:voice/v2/service\0plasmo:voice/v2");
        let control = super::plugin_packet("plasmo:voice/v2", &[0, 1, 255]).unwrap();
        assert_eq!(control.body, b"\x0fplasmo:voice/v2\0\x01\xff");
        assert!(super::plugin_packet("plasmo:voice/v2", &vec![0; 32768]).is_err());
    }

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
