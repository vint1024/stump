//! E3 offline content protection: wrap a book's bytes so that only a registered
//! device (holding the matching Secure Enclave P-256 private key) can decrypt
//! them. ECIES over P-256: ephemeral ECDH → HKDF-SHA256 → AES-256-GCM. The wire
//! format matches the Swift client (CryptoKit) and the cross-validated reference
//! in `noirpanther/docs/OFFLINE_ENCRYPTION.md`:
//!   - AES-256-GCM combined box = nonce(12) ‖ ciphertext ‖ tag(16)
//!   - HKDF-SHA256(ikm = ECDH shared X, salt = SALT, info = INFO, L = 32)
//!   - ephemeral / device public keys are X9.63 uncompressed points (65 bytes)
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use hkdf::Hkdf;
use p256::ecdh::EphemeralSecret;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::PublicKey;
use sha2::Sha256;

const SALT: &[u8] = b"noirpanther-offline-salt-v1";
const INFO: &[u8] = b"noirpanther-offline-v1";

/// The result of wrapping a book for offline reading on a specific device.
pub struct WrappedContent {
	/// AES-256-GCM(content_key, plaintext) — the encrypted book.
	pub blob: Vec<u8>,
	/// The ephemeral public key (X9.63 uncompressed, 65 bytes) used for ECDH.
	pub ephemeral_pub: Vec<u8>,
	/// AES-256-GCM(kek, content_key) — the content key wrapped to the device.
	pub wrapped_key: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum OfflineCryptoError {
	#[error("invalid device public key")]
	BadDeviceKey,
	#[error("encryption failed")]
	Encrypt,
	#[error("key derivation failed")]
	Kdf,
}

/// Validate that `device_pub_x963` is a real P-256 point (uncompressed SEC1, 65
/// bytes) and not the identity. Lets `register_device` reject a malformed key up
/// front (400) instead of failing later inside `wrap_for_device` (500).
pub fn validate_device_key(device_pub_x963: &[u8]) -> Result<(), OfflineCryptoError> {
	PublicKey::from_sec1_bytes(device_pub_x963)
		.map(|_| ())
		.map_err(|_| OfflineCryptoError::BadDeviceKey)
}

fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, OfflineCryptoError> {
	let cipher = Aes256Gcm::new(key.into());
	let mut nonce_bytes = [0u8; 12];
	OsRng.fill_bytes(&mut nonce_bytes);
	let ciphertext = cipher
		.encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
		.map_err(|_| OfflineCryptoError::Encrypt)?;
	let mut out = nonce_bytes.to_vec();
	out.extend(ciphertext);
	Ok(out)
}

/// Encrypt `plaintext` for a device whose P-256 public key is `device_pub_x963`
/// (uncompressed SEC1 / X9.63, 65 bytes). The returned content key is wrapped so
/// only that device's private key can recover it; everything else on the wire is
/// useless without the device.
pub fn wrap_for_device(
	plaintext: &[u8],
	device_pub_x963: &[u8],
) -> Result<WrappedContent, OfflineCryptoError> {
	let device_pub = PublicKey::from_sec1_bytes(device_pub_x963)
		.map_err(|_| OfflineCryptoError::BadDeviceKey)?;

	let mut content_key = [0u8; 32];
	OsRng.fill_bytes(&mut content_key);
	let blob = seal(&content_key, plaintext)?;

	let ephemeral = EphemeralSecret::random(&mut OsRng);
	let ephemeral_pub = ephemeral
		.public_key()
		.to_encoded_point(false)
		.as_bytes()
		.to_vec();
	let shared = ephemeral.diffie_hellman(&device_pub);
	let hkdf = Hkdf::<Sha256>::new(Some(SALT), &shared.raw_secret_bytes()[..]);
	let mut kek = [0u8; 32];
	hkdf.expand(INFO, &mut kek)
		.map_err(|_| OfflineCryptoError::Kdf)?;
	let wrapped_key = seal(&kek, &content_key)?;

	Ok(WrappedContent {
		blob,
		ephemeral_pub,
		wrapped_key,
	})
}
