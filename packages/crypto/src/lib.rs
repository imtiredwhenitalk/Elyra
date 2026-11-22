use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hpke::{
    aead::AesGcm256,
    kdf::HkdfSha256,
    kem::X25519HkdfSha256,
    Deserializable, Kem as KemTrait, OpModeR, OpModeS, Serializable,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;
const X25519_KEY_SIZE: usize = 32;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key must be exactly 32 bytes")]
    InvalidKey,
    #[error("key or payload is not valid base64")]
    InvalidEncoding,
    #[error("encrypted payload is incomplete")]
    IncompletePayload,
    #[error("authenticated encryption failed")]
    EncryptionFailed,
    #[error("decrypted data is not valid UTF-8")]
    InvalidUtf8,
    #[error("HPKE recipient envelope failed")]
    HpkeFailed,
    #[error("message envelope is not valid JSON")]
    EnvelopeEncoding,
    #[error("unsupported message envelope version")]
    UnsupportedEnvelope,
    #[error("Ed25519 signature is invalid")]
    InvalidSignature,
}

#[derive(Clone)]
pub struct EncryptionKey(Zeroizing<[u8; KEY_SIZE]>);

#[derive(Clone)]
pub struct X25519KeyPair(Zeroizing<[u8; X25519_KEY_SIZE]>);

impl X25519KeyPair {
    pub fn generate() -> Self {
        loop {
            let mut bytes = [0_u8; X25519_KEY_SIZE];
            OsRng.fill_bytes(&mut bytes);
            if bytes.iter().any(|byte| *byte != 0) {
                let pair = Self(Zeroizing::new(bytes));
                if pair.public_key_bytes().iter().any(|byte| *byte != 0) {
                    return pair;
                }
            }
        }
    }

    pub fn from_base64(encoded: &str) -> Result<Self, CryptoError> {
        let bytes = Zeroizing::new(
            BASE64
                .decode(encoded)
                .map_err(|_| CryptoError::InvalidEncoding)?,
        );
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidKey);
        }
        if bytes.len() != X25519_KEY_SIZE {
            return Err(CryptoError::InvalidKey);
        }
        let mut private_key = [0_u8; X25519_KEY_SIZE];
        private_key.copy_from_slice(bytes.as_slice());
        let pair = Self(Zeroizing::new(private_key));
        if pair.public_key_bytes().iter().all(|byte| *byte == 0) {
            return Err(CryptoError::InvalidKey);
        }
        Ok(pair)
    }

    pub fn private_key_base64(&self) -> Zeroizing<String> {
        Zeroizing::new(BASE64.encode(self.0.as_slice()))
    }

    pub fn public_key_bytes(&self) -> [u8; X25519_KEY_SIZE] {
        let secret = StaticSecret::from(*self.0);
        PublicKey::from(&secret).to_bytes()
    }

    pub fn public_key_base64(&self) -> String {
        BASE64.encode(self.public_key_bytes())
    }
}

impl EncryptionKey {
    pub fn generate() -> Self {
        let mut bytes = [0_u8; KEY_SIZE];
        OsRng.fill_bytes(&mut bytes);
        Self(Zeroizing::new(bytes))
    }

    pub fn from_base64(encoded: &str) -> Result<Self, CryptoError> {
        let bytes = BASE64
            .decode(encoded)
            .map_err(|_| CryptoError::InvalidEncoding)?;
        let key = bytes.try_into().map_err(|_| CryptoError::InvalidKey)?;
        Ok(Self(Zeroizing::new(key)))
    }

    pub fn to_base64(&self) -> String {
        BASE64.encode(self.0.as_slice())
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.0.as_slice()))
    }
}

pub fn encrypt(key: &EncryptionKey, plaintext: &[u8]) -> Result<String, CryptoError> {
    let mut nonce_bytes = [0_u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = key
        .cipher()
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let mut payload = nonce_bytes.to_vec();
    payload.extend(ciphertext);
    Ok(BASE64.encode(payload))
}

pub fn decrypt(key: &EncryptionKey, encoded: &str) -> Result<Vec<u8>, CryptoError> {
    let payload = BASE64
        .decode(encoded)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    if payload.len() <= NONCE_SIZE {
        return Err(CryptoError::IncompletePayload);
    }
    key.cipher()
        .decrypt(
            Nonce::from_slice(&payload[..NONCE_SIZE]),
            &payload[NONCE_SIZE..],
        )
        .map_err(|_| CryptoError::EncryptionFailed)
}

pub fn decrypt_text(key: &EncryptionKey, encoded: &str) -> Result<String, CryptoError> {
    String::from_utf8(decrypt(key, encoded)?).map_err(|_| CryptoError::InvalidUtf8)
}

pub fn fingerprint(value: &[u8]) -> String {
    Sha256::digest(value)
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(":")
}

const ENVELOPE_VERSION: u8 = 1;
const ENVELOPE_INFO: &[u8] = b"Elyra message envelope v1";
const PAYLOAD_ID_SIZE: usize = 32;

type HpkeKem = X25519HkdfSha256;
type HpkeKdf = HkdfSha256;
type HpkeAead = AesGcm256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecipientEnvelope {
    pub recipient_key_id: String,
    pub encapsulated_key: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageEnvelope {
    pub version: u8,
    pub sender_identity_key_id: String,
    pub sender_signature: String,
    pub recipient_key_ids: Vec<String>,
    pub recipient_ciphertexts: Vec<RecipientEnvelope>,
    pub payload_id: String,
    pub encrypted_payload_nonce: String,
    pub payload_ciphertext: String,
}

impl MessageEnvelope {
    pub fn to_json(&self) -> Result<String, CryptoError> {
        serde_json::to_string(self).map_err(|_| CryptoError::EnvelopeEncoding)
    }

    pub fn from_json(encoded: &str) -> Result<Self, CryptoError> {
        let envelope: Self =
            serde_json::from_str(encoded).map_err(|_| CryptoError::EnvelopeEncoding)?;
        if envelope.version != ENVELOPE_VERSION {
            return Err(CryptoError::UnsupportedEnvelope);
        }
        Ok(envelope)
    }

    pub fn canonical_aad(&self) -> Result<Vec<u8>, CryptoError> {
        let metadata = (&self.version,
            &self.sender_identity_key_id,
            &self.recipient_key_ids,
            &self.recipient_ciphertexts,
            &self.payload_id,
            &self.encrypted_payload_nonce);
        serde_json::to_vec(&metadata).map_err(|_| CryptoError::EnvelopeEncoding)
    }
}

fn canonical_signed_bytes(envelope: &MessageEnvelope) -> Result<Vec<u8>, CryptoError> {
    let mut signed = envelope.canonical_aad()?;
    let ciphertext = envelope.payload_ciphertext.as_bytes();
    signed.extend_from_slice(&(ciphertext.len() as u64).to_be_bytes());
    signed.extend_from_slice(ciphertext);
    Ok(signed)
}

pub fn seal_payload_key_to_recipient(
    recipient_key_id: &str,
    recipient_public_key: &str,
    payload_key: &[u8],
) -> Result<RecipientEnvelope, CryptoError> {
    if recipient_key_id.is_empty() {
        return Err(CryptoError::InvalidKey);
    }
    if payload_key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidKey);
    }
    let public_bytes = BASE64
        .decode(recipient_public_key)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    if public_bytes.len() != X25519_KEY_SIZE {
        return Err(CryptoError::InvalidKey);
    }
    let public_key = <HpkeKem as KemTrait>::PublicKey::from_bytes(&public_bytes)
        .map_err(|_| CryptoError::InvalidKey)?;
    let (encapsulated_key, ciphertext) =
        hpke::single_shot_seal::<HpkeAead, HpkeKdf, HpkeKem>(
            &OpModeS::Base,
            &public_key,
            ENVELOPE_INFO,
            payload_key,
            b"Elyra recipient envelope v1",
        )
        .map_err(|_| CryptoError::HpkeFailed)?;
    let encapsulated_key = encapsulated_key.to_bytes();
    Ok(RecipientEnvelope {
        recipient_key_id: recipient_key_id.to_string(),
        encapsulated_key: BASE64.encode(encapsulated_key),
        ciphertext: BASE64.encode(ciphertext),
    })
}

pub fn open_payload_key_for_recipient(
    recipient_private_key: &str,
    recipient: &RecipientEnvelope,
) -> Result<Vec<u8>, CryptoError> {
    let private_bytes = BASE64
        .decode(recipient_private_key)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    if private_bytes.len() != X25519_KEY_SIZE {
        return Err(CryptoError::InvalidKey);
    }
    let private_key = <HpkeKem as KemTrait>::PrivateKey::from_bytes(&private_bytes)
        .map_err(|_| CryptoError::InvalidKey)?;
    let encapsulated_bytes = BASE64
        .decode(&recipient.encapsulated_key)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    let encapsulated_key =
        <HpkeKem as KemTrait>::EncappedKey::from_bytes(&encapsulated_bytes)
            .map_err(|_| CryptoError::InvalidEncoding)?;
    let ciphertext = BASE64
        .decode(&recipient.ciphertext)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    hpke::single_shot_open::<HpkeAead, HpkeKdf, HpkeKem>(
        &OpModeR::Base,
        &private_key,
        &encapsulated_key,
        ENVELOPE_INFO,
        &ciphertext,
        b"Elyra recipient envelope v1",
    )
    .map_err(|_| CryptoError::HpkeFailed)
}

pub fn sign_message_envelope(
    envelope: &mut MessageEnvelope,
    sender_private_key: &str,
) -> Result<(), CryptoError> {
    let private_bytes = BASE64
        .decode(sender_private_key)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    let private_array: [u8; 32] = private_bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidKey)?;
    let signing_key = SigningKey::from_bytes(&private_array);
    let signature = signing_key.sign(&canonical_signed_bytes(envelope)?);
    envelope.sender_signature = BASE64.encode(signature.to_bytes());
    Ok(())
}

pub fn verify_message_envelope(
    envelope: &MessageEnvelope,
    sender_public_key: &str,
) -> Result<(), CryptoError> {
    let public_bytes = BASE64
        .decode(sender_public_key)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    let public_array: [u8; 32] = public_bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidKey)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_array).map_err(|_| CryptoError::InvalidKey)?;
    let signature_bytes = BASE64
        .decode(&envelope.sender_signature)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidSignature)?;
    let signature = Signature::from_bytes(&signature_array);
    verifying_key
        .verify_strict(&canonical_signed_bytes(envelope)?, &signature)
        .map_err(|_| CryptoError::InvalidSignature)
}

#[derive(Debug, Clone)]
pub struct RecipientPublicKey {
    pub key_id: String,
    pub public_key: String,
}

pub fn seal_message_envelope(
    sender_identity_key_id: &str,
    sender_private_key: &str,
    recipients: &[RecipientPublicKey],
    plaintext: &[u8],
) -> Result<MessageEnvelope, CryptoError> {
    if sender_identity_key_id.is_empty() || recipients.is_empty() {
        return Err(CryptoError::InvalidKey);
    }
    let payload_key = EncryptionKey::generate();
    let mut recipient_ciphertexts = Vec::with_capacity(recipients.len());
    let mut recipient_key_ids = Vec::with_capacity(recipients.len());
    for recipient in recipients {
        recipient_ciphertexts.push(seal_payload_key_to_recipient(
            &recipient.key_id,
            &recipient.public_key,
            payload_key.0.as_slice(),
        )?);
        recipient_key_ids.push(recipient.key_id.clone());
    }
    let mut nonce_bytes = [0_u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let mut payload_id_bytes = [0_u8; PAYLOAD_ID_SIZE];
    OsRng.fill_bytes(&mut payload_id_bytes);
    let mut envelope = MessageEnvelope {
        version: ENVELOPE_VERSION,
        sender_identity_key_id: sender_identity_key_id.to_string(),
        sender_signature: String::new(),
        recipient_key_ids,
        recipient_ciphertexts,
        payload_id: BASE64.encode(payload_id_bytes),
        encrypted_payload_nonce: BASE64.encode(nonce_bytes),
        payload_ciphertext: String::new(),
    };
    let aad = envelope.canonical_aad()?;
    let ciphertext = payload_key
        .cipher()
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            aes_gcm::aead::Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)?;
    envelope.payload_ciphertext = BASE64.encode(ciphertext);
    sign_message_envelope(&mut envelope, sender_private_key)?;
    Ok(envelope)
}

pub fn open_message_envelope(
    envelope: &MessageEnvelope,
    recipient_key_id: &str,
    recipient_private_key: &str,
    sender_public_key: &str,
) -> Result<Vec<u8>, CryptoError> {
    if envelope.version != ENVELOPE_VERSION {
        return Err(CryptoError::UnsupportedEnvelope);
    }
    verify_message_envelope(envelope, sender_public_key)?;
    let recipient = envelope
        .recipient_ciphertexts
        .iter()
        .find(|item| item.recipient_key_id == recipient_key_id)
        .ok_or(CryptoError::InvalidKey)?;
    let payload_key_bytes =
        open_payload_key_for_recipient(recipient_private_key, recipient)?;
    let payload_key = EncryptionKey::from_base64(&BASE64.encode(payload_key_bytes))?;
    let nonce_bytes = BASE64
        .decode(&envelope.encrypted_payload_nonce)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    if nonce_bytes.len() != NONCE_SIZE {
        return Err(CryptoError::IncompletePayload);
    }
    let ciphertext = BASE64
        .decode(&envelope.payload_ciphertext)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    payload_key
        .cipher()
        .decrypt(
            Nonce::from_slice(&nonce_bytes),
            aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: &envelope.canonical_aad()?,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_for_text_and_empty_data() {
        let key = EncryptionKey::generate();
        let encrypted = encrypt(&key, b"private message").expect("encryption");
        assert_eq!(
            decrypt_text(&key, &encrypted).expect("decryption"),
            "private message"
        );

        let encrypted_empty = encrypt(&key, b"").expect("empty encryption");
        assert_eq!(
            decrypt(&key, &encrypted_empty).expect("empty decryption"),
            b""
        );
    }

    #[test]
    fn rejects_invalid_keys_and_encoding() {
        for encoded in ["", "aA==", "aW52YWxpZA=="] {
            assert!(matches!(
                EncryptionKey::from_base64(encoded),
                Err(CryptoError::InvalidKey | CryptoError::InvalidEncoding)
            ));
        }

        assert!(matches!(
            EncryptionKey::from_base64("not base64 ***"),
            Err(CryptoError::InvalidEncoding)
        ));

        let long_key = vec![0_u8; KEY_SIZE + 1];
        let encoded_long_key = BASE64.encode(&long_key);
        assert!(matches!(
            EncryptionKey::from_base64(&encoded_long_key),
            Err(CryptoError::InvalidKey)
        ));

        let short_key = vec![0_u8; KEY_SIZE - 1];
        let encoded_short_key = BASE64.encode(&short_key);
        assert!(matches!(
            EncryptionKey::from_base64(&encoded_short_key),
            Err(CryptoError::InvalidKey)
        ));
    }

    #[test]
    fn rejects_malformed_payloads() {
        let key = EncryptionKey::generate();

        for encoded in [
            "",
            "bm90IGJhc2U2NA==!",
            &BASE64.encode([]),
            &BASE64.encode([0_u8; NONCE_SIZE]),
            &BASE64.encode([0_u8; NONCE_SIZE - 1]),
            &BASE64.encode([0_u8; NONCE_SIZE + 1]),
        ] {
            assert!(decrypt(&key, encoded).is_err());
        }
    }

    #[test]
    fn rejects_tampering_wrong_key_and_truncated_payload() {
        let key = EncryptionKey::generate();
        let encrypted = encrypt(&key, b"authenticated data").expect("encryption");
        let mut payload = BASE64.decode(&encrypted).expect("payload");

        payload[0] ^= 1;
        assert!(matches!(
            decrypt(&key, &BASE64.encode(&payload)),
            Err(CryptoError::EncryptionFailed)
        ));

        payload = BASE64.decode(&encrypted).expect("payload");
        payload[NONCE_SIZE] ^= 1;
        assert!(matches!(
            decrypt(&key, &BASE64.encode(&payload)),
            Err(CryptoError::EncryptionFailed)
        ));

        payload.pop();
        assert!(matches!(
            decrypt(&key, &BASE64.encode(&payload)),
            Err(CryptoError::EncryptionFailed)
        ));

        payload = BASE64.decode(&encrypted).expect("payload");
        payload.push(0);
        assert!(matches!(
            decrypt(&key, &BASE64.encode(&payload)),
            Err(CryptoError::EncryptionFailed)
        ));

        let different_key = EncryptionKey::generate();
        assert!(matches!(
            decrypt(&different_key, &encrypted),
            Err(CryptoError::EncryptionFailed)
        ));
    }

    #[test]
    fn encryption_is_nonce_unique_and_text_utf8_is_validated() {
        let key = EncryptionKey::generate();
        let first = encrypt(&key, b"same plaintext").expect("first encryption");
        let second = encrypt(&key, b"same plaintext").expect("second encryption");
        assert_ne!(first, second);

        let binary = vec![0_u8, 255, 128, 1, 200];
        let encrypted = encrypt(&key, &binary).expect("binary encryption");
        assert_eq!(
            decrypt(&key, &encrypted).expect("binary decryption"),
            binary
        );

        let invalid_utf8 = encrypt(&key, &[0xff, 0xfe]).expect("invalid UTF-8 encryption");
        assert!(matches!(
            decrypt_text(&key, &invalid_utf8),
            Err(CryptoError::InvalidUtf8)
        ));
    }

    #[test]
    fn x25519_keypair_round_trips_private_key_and_derives_public_key() {
        let pair = X25519KeyPair::generate();
        let encoded_private = pair.private_key_base64();
        let restored =
            X25519KeyPair::from_base64(encoded_private.as_str()).expect("valid private key");

        assert_eq!(
            restored.private_key_base64().as_str(),
            encoded_private.as_str()
        );
        assert_eq!(restored.public_key_base64(), pair.public_key_base64());
        assert_ne!(pair.public_key_bytes(), [0_u8; X25519_KEY_SIZE]);
    }

    #[test]
    fn x25519_keypair_rejects_invalid_private_keys() {
        assert!(matches!(
            X25519KeyPair::from_base64(""),
            Err(CryptoError::InvalidKey)
        ));
        assert!(matches!(
            X25519KeyPair::from_base64("not base64 ***"),
            Err(CryptoError::InvalidEncoding)
        ));

        let zero_key = BASE64.encode([0_u8; X25519_KEY_SIZE]);
        assert!(matches!(
            X25519KeyPair::from_base64(&zero_key),
            Err(CryptoError::InvalidKey)
        ));

        let short_key = BASE64.encode([0_u8; X25519_KEY_SIZE - 1]);
        assert!(matches!(
            X25519KeyPair::from_base64(&short_key),
            Err(CryptoError::InvalidKey)
        ));

        let long_key = BASE64.encode([0_u8; X25519_KEY_SIZE + 1]);
        assert!(matches!(
            X25519KeyPair::from_base64(&long_key),
            Err(CryptoError::InvalidKey)
        ));
    }

    #[test]
    fn hpke_recipient_round_trip_and_ed25519_signature() {
        let recipient = X25519KeyPair::generate();
        let recipient_private = recipient.private_key_base64();
        let recipient_public = recipient.public_key_base64();
        let payload_key = [4_u8; KEY_SIZE];
        let recipient_envelope = seal_payload_key_to_recipient(
            "recipient-key-1",
            &recipient_public,
            &payload_key,
        )
        .expect("recipient sealing");
        let opened = open_payload_key_for_recipient(
            recipient_private.as_str(),
            &recipient_envelope,
        )
        .expect("recipient opening");
        assert_eq!(opened, payload_key);

        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = BASE64.encode(signing_key.verifying_key().to_bytes());
        let private_key = BASE64.encode(signing_key.to_bytes());
        let mut envelope = MessageEnvelope {
            version: ENVELOPE_VERSION,
            sender_identity_key_id: "identity-key-1".to_string(),
            sender_signature: String::new(),
            recipient_key_ids: vec!["recipient-key-1".to_string()],
            recipient_ciphertexts: vec![recipient_envelope],
            payload_id: "payload-1".to_string(),
            encrypted_payload_nonce: BASE64.encode([7_u8; NONCE_SIZE]),
            payload_ciphertext: BASE64.encode(b"ciphertext"),
        };
        sign_message_envelope(&mut envelope, &private_key).expect("sign envelope");
        verify_message_envelope(&envelope, &public_key).expect("verify envelope");

        envelope.payload_ciphertext = BASE64.encode(b"tampered");
        assert!(matches!(
            verify_message_envelope(&envelope, &public_key),
            Err(CryptoError::InvalidSignature)
        ));
    }

    #[test]
    fn hpke_rejects_wrong_recipient_and_malformed_envelope() {
        let recipient = X25519KeyPair::generate();
        let wrong_recipient = X25519KeyPair::generate();
        let recipient_envelope = seal_payload_key_to_recipient(
            "recipient-key-1",
            &recipient.public_key_base64(),
            &[3_u8; KEY_SIZE],
        )
        .expect("recipient sealing");
        assert!(open_payload_key_for_recipient(
            wrong_recipient.private_key_base64().as_str(),
            &recipient_envelope,
        )
        .is_err());

        assert!(matches!(
            MessageEnvelope::from_json(
                r#"{"version":99,"senderIdentityKeyId":"","senderSignature":"","recipientKeyIds":[],"recipientCiphertexts":[],"payloadId":"","encryptedPayloadNonce":"","payloadCiphertext":""}"#
            ),
            Err(CryptoError::UnsupportedEnvelope)
        ));
    }

    #[test]
    fn seals_and_opens_a_signed_typed_message_envelope() {
        let recipient = X25519KeyPair::generate();
        let sender = SigningKey::generate(&mut OsRng);
        let envelope = seal_message_envelope(
            "identity-key-1",
            &BASE64.encode(sender.to_bytes()),
            &[RecipientPublicKey {
                key_id: "recipient-key-1".to_string(),
                public_key: recipient.public_key_base64(),
            }],
            b"encrypted Elyra message",
        )
        .expect("seal message");

        assert_eq!(envelope.version, ENVELOPE_VERSION);
        assert_eq!(envelope.recipient_key_ids, vec!["recipient-key-1"]);
        let opened = open_message_envelope(
            &envelope,
            "recipient-key-1",
            recipient.private_key_base64().as_str(),
            &BASE64.encode(sender.verifying_key().to_bytes()),
        )
        .expect("open message");
        assert_eq!(opened, b"encrypted Elyra message");

        let wrong_recipient = X25519KeyPair::generate();
        assert!(open_message_envelope(
            &envelope,
            "recipient-key-1",
            wrong_recipient.private_key_base64().as_str(),
            &BASE64.encode(sender.verifying_key().to_bytes()),
        )
        .is_err());
    }

    mod proptest {
        use super::*;
        use ::proptest::collection;
        use ::proptest::prelude::*;

        proptest! {
            #[test]
            fn binary_round_trip_for_arbitrary_data(data in collection::vec(any::<u8>(), 0..4096)) {
                let key = EncryptionKey::generate();
                let encrypted = encrypt(&key, &data).expect("property encryption");
                let decrypted = decrypt(&key, &encrypted).expect("property decryption");
                prop_assert_eq!(decrypted, data);
            }

            #[test]
            fn arbitrary_payload_bytes_never_panic(payload in collection::vec(any::<u8>(), 0..4096)) {
                let key = EncryptionKey::generate();
                let encoded = BASE64.encode(payload);
                let _ = decrypt(&key, &encoded);
            }

            #[test]
            fn arbitrary_base64_text_never_panics(text in "[ -~]{0,256}") {
                let key = EncryptionKey::generate();
                let _ = decrypt(&key, &text);
            }

            #[test]
            fn arbitrary_invalid_key_lengths_are_rejected(length in 0usize..128) {
                prop_assume!(length != KEY_SIZE);
                let bytes = vec![0_u8; length];
                let encoded = BASE64.encode(bytes);
                prop_assert!(matches!(
                    EncryptionKey::from_base64(&encoded),
                    Err(CryptoError::InvalidKey)
                ));
            }
        }
    }
}
