//! WireGuard key handling, implemented entirely in Rust.
//!
//! WireGuard keys are X25519 scalars encoded as 32 raw bytes in standard
//! base64.  We use `x25519-dalek` for the scalar multiplication instead of
//! shelling out to `wg`, `openssl`, or any other system tool.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand_core::{OsRng, RngCore};
use x25519_dalek::{PublicKey, StaticSecret};

const KEY_LEN: usize = 32;

/// Generate a new WireGuard private key (clamped X25519 scalar, base64).
pub fn generate_private_key() -> String {
    let mut bytes = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut bytes);
    clamp_private_key(&mut bytes);
    STANDARD.encode(bytes)
}

/// Generate a WireGuard preshared key (32 random bytes, base64).
pub fn generate_preshared_key() -> String {
    let mut bytes = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut bytes);
    STANDARD.encode(bytes)
}

/// Derive the public key belonging to a base64 encoded private key.
pub fn public_key(private_key: &str) -> Result<String, String> {
    let bytes = decode_key(private_key, "private key")?;
    let secret = StaticSecret::from(bytes);
    let public = PublicKey::from(&secret);
    Ok(STANDARD.encode(public.to_bytes()))
}

/// Decode and validate a base64 key.  Declared public so the UI can validate
/// imported projects without duplicating the error messages.
pub fn decode_key(value: &str, label: &str) -> Result<[u8; KEY_LEN], String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} is empty"));
    }

    let bytes = STANDARD
        .decode(value)
        .map_err(|error| format!("{label} is not valid base64: {error}"))?;

    bytes
        .try_into()
        .map_err(|_| format!("{label} must decode to exactly {KEY_LEN} bytes"))
}

pub fn is_valid_key(value: &str) -> bool {
    decode_key(value, "key").is_ok()
}

/// Apply the same bit clamping `wg genkey` / X25519 uses for private scalars.
fn clamp_private_key(key: &mut [u8; KEY_LEN]) {
    key[0] &= 248;
    key[31] &= 127;
    key[31] |= 64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_clamped_and_valid() {
        for _ in 0..16 {
            let private = generate_private_key();
            let bytes = decode_key(&private, "private key").unwrap();
            assert_eq!(bytes[0] & 0b0000_0111, 0);
            assert_eq!(bytes[31] & 0b1000_0000, 0);
            assert_eq!(bytes[31] & 0b0100_0000, 0b0100_0000);
            assert!(public_key(&private).is_ok());
        }
    }

    #[test]
    fn rfc7748_alice_key_vector() {
        // RFC 7748 section 6.1.
        let private = "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a";
        let private_b64 = STANDARD.encode(decode_hex(private));
        let expected = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a";
        assert_eq!(
            public_key(&private_b64).unwrap(),
            STANDARD.encode(decode_hex(expected))
        );
    }

    #[test]
    fn rejects_bad_keys() {
        assert!(public_key("not base64!").is_err());
        assert!(public_key("AAAA").is_err());
        assert!(public_key("").is_err());
    }

    fn decode_hex(input: &str) -> Vec<u8> {
        input
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }
}
