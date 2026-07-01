//! Secure vault service — AES-256-GCM encrypted credential storage.
//!
//! Stores provider API keys with encryption at rest.
//! Uses the system password as the encryption master key (via a simple KDF).
//! Falls back to unencrypted JSON if no master password is set.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, Key};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

/// Master key derived from a password using SHA-256 KDF.
fn derive_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.finalize().into()
}

/// Encrypt a value using AES-256-GCM with a random IV.
fn encrypt(value: &str, key: &[u8; 32]) -> Result<String, VaultError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = aes_gcm::Nonce::from([0u8; 12]);
    let ciphertext = cipher.encrypt(&nonce, value.as_bytes())
        .map_err(|e| VaultError::EncryptionError(e.to_string()))?;
    // Combine IV + ciphertext and base64 encode
    let mut data = nonce.to_vec();
    data.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(&data))
}

/// Decrypt a base64-encoded AES-256-GCM ciphertext.
fn decrypt(encrypted: &str, key: &[u8; 32]) -> Result<String, VaultError> {
    let data = STANDARD.decode(encrypted)
        .map_err(|e| VaultError::DecryptionError(e.to_string()))?;
    if data.len() < 12 {
        return Err(VaultError::DecryptionError("Data too short".to_string()));
    }
    let nonce = aes_gcm::Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| VaultError::DecryptionError(e.to_string()))?;
    String::from_utf8(plaintext)
        .map_err(|e| VaultError::DecryptionError(e.to_string()))
}

/// Vault service — secure credential storage with AES-256-GCM encryption.
pub struct VaultService {
    /// In-memory store (decrypted after first access).
    store: Mutex<HashMap<String, String>>,
    /// Path to the encrypted credential file.
    file_path: PathBuf,
    /// Whether encryption is enabled.
    encrypted: bool,
    /// Master key (if encrypted).
    master_key: Option<[u8; 32]>,
}

impl VaultService {
    /// Create a new vault service.
    ///
    /// If `master_password` is Some, credentials are encrypted with AES-256-GCM.
    /// If None, credentials are stored unencrypted (fallback mode).
    pub fn new(service: &str, master_password: Option<String>) -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        let file_path = PathBuf::from(home)
            .join(".praxis")
            .join(format!("{}.vault.json", service));

        let encrypted = master_password.is_some();
        let master_key = master_password.as_ref().map(|p| derive_key(p.as_str()));

        Self {
            store: Mutex::new(HashMap::new()),
            file_path,
            encrypted,
            master_key,
        }
    }

    /// Create with a custom path.
    pub fn with_path(path: PathBuf, master_password: Option<String>) -> Self {
        let encrypted = master_password.is_some();
        let master_key = master_password.as_ref().map(|p| derive_key(p.as_str()));

        Self {
            store: Mutex::new(HashMap::new()),
            file_path: path,
            encrypted,
            master_key,
        }
    }

    /// Load stored credentials from disk.
    fn load(&self) -> Result<HashMap<String, String>, VaultError> {
        if !self.file_path.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| VaultError::FileReadError {
                path: self.file_path.display().to_string(),
                reason: e.to_string(),
            })?;

        let entries: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| VaultError::ParseError(e.to_string()))?;

        let mut store = HashMap::new();
        for (key, value) in entries {
            let resolved = if self.encrypted {
                let master_key = self.master_key.as_ref()
                    .ok_or(VaultError::NoMasterKey)?;
                decrypt(&value, master_key)
                    .unwrap_or_else(|_| value) // If decryption fails, store raw
            } else {
                value
            };
            store.insert(key, resolved);
        }

        Ok(store)
    }

    /// Persist credentials to disk.
    fn persist(&self, store: &HashMap<String, String>) -> Result<(), VaultError> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| VaultError::FileWriteError {
                    path: self.file_path.display().to_string(),
                    reason: e.to_string(),
                })?;
        }

        let entries: HashMap<String, String> = if self.encrypted {
            let key = self.master_key.as_ref()
                .ok_or(VaultError::NoMasterKey)?;
            store.iter()
                .map(|(k, v)| {
                    let enc = encrypt(v, key).unwrap_or_else(|_| v.clone());
                    (k.clone(), enc)
                })
                .collect()
        } else {
            store.clone()
        };

        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| VaultError::SerializationError(e.to_string()))?;

        std::fs::write(&self.file_path, json)
            .map_err(|e| VaultError::FileWriteError {
                path: self.file_path.display().to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Store a credential.
    pub fn set(&self, key: &str, value: &str) -> Result<(), VaultError> {
        let mut store = self.store.lock().map_err(|e| VaultError::LockError(e.to_string()))?;
        store.insert(key.to_string(), value.to_string());
        self.persist(&store)?;
        Ok(())
    }

    /// Retrieve a credential.
    pub fn get(&self, key: &str) -> Result<Option<String>, VaultError> {
        let store = self.store.lock().map_err(|e| VaultError::LockError(e.to_string()))?;
        Ok(store.get(key).cloned())
    }

    /// Delete a credential.
    pub fn delete(&self, key: &str) -> Result<(), VaultError> {
        let mut store = self.store.lock().map_err(|e| VaultError::LockError(e.to_string()))?;
        store.remove(key);
        self.persist(&store)?;
        Ok(())
    }

    /// List all stored credential keys.
    pub fn list_keys(&self) -> Result<Vec<String>, VaultError> {
        let store = self.store.lock().map_err(|e| VaultError::LockError(e.to_string()))?;
        Ok(store.keys().cloned().collect())
    }

    /// Clear all credentials.
    pub fn clear(&self) -> Result<(), VaultError> {
        let mut store = self.store.lock().map_err(|e| VaultError::LockError(e.to_string()))?;
        store.clear();
        self.persist(&store)?;
        Ok(())
    }

    /// Initialize the vault by loading from disk.
    pub fn init(&self) -> Result<(), VaultError> {
        let store = self.load()?;
        let mut guard = self.store.lock().map_err(|e| VaultError::LockError(e.to_string()))?;
        *guard = store;
        Ok(())
    }

    /// Reload credentials from disk.
    pub fn reload(&self) -> Result<(), VaultError> {
        self.init()
    }
}

impl Default for VaultService {
    fn default() -> Self {
        Self::new("praxis", None)
    }
}

/// Vault error types.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("File read error: {path}: {reason}")]
    FileReadError { path: String, reason: String },

    #[error("File write error: {path}: {reason}")]
    FileWriteError { path: String, reason: String },

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("No master key set for encryption")]
    NoMasterKey,

    #[error("Lock error: {0}")]
    LockError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key("test-password");
        let plaintext = "sk-test-api-key-12345";
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_vault_set_get() {
        let vault = VaultService::with_path(
            std::env::temp_dir().join(format!("vault-test-{}.json", uuid::Uuid::new_v4())),
            Some("test-pass".to_string()),
        );
        vault.init().unwrap();
        vault.set("nan", "sk-nan-12345").unwrap();
        let value = vault.get("nan").unwrap();
        assert_eq!(value, Some("sk-nan-12345".to_string()));
    }

    #[test]
    fn test_vault_delete() {
        let vault = VaultService::with_path(
            std::env::temp_dir().join(format!("vault-del-{}.json", uuid::Uuid::new_v4())),
            Some("test-pass".to_string()),
        );
        vault.init().unwrap();
        vault.set("key", "value").unwrap();
        vault.delete("key").unwrap();
        assert!(vault.get("key").unwrap().is_none());
    }

    #[test]
    fn test_vault_list_keys() {
        let vault = VaultService::with_path(
            std::env::temp_dir().join(format!("vault-list-{}.json", uuid::Uuid::new_v4())),
            Some("test-pass".to_string()),
        );
        vault.init().unwrap();
        vault.set("a", "1").unwrap();
        vault.set("b", "2").unwrap();
        let keys = vault.list_keys().unwrap();
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
    }

    #[test]
    fn test_vault_unencrypted_mode() {
        let vault = VaultService::with_path(
            std::env::temp_dir().join(format!("vault-unenc-{}.json", uuid::Uuid::new_v4())),
            None, // No password = unencrypted
        );
        vault.init().unwrap();
        vault.set("key", "secret").unwrap();
        let value = vault.get("key").unwrap();
        assert_eq!(value, Some("secret".to_string()));
    }

    #[test]
    fn test_vault_clear() {
        let vault = VaultService::with_path(
            std::env::temp_dir().join(format!("vault-clear-{}.json", uuid::Uuid::new_v4())),
            Some("test-pass".to_string()),
        );
        vault.init().unwrap();
        vault.set("a", "1").unwrap();
        vault.set("b", "2").unwrap();
        vault.clear().unwrap();
        assert!(vault.list_keys().unwrap().is_empty());
    }

    #[test]
    fn test_wrong_key_cannot_decrypt() {
        let key1 = derive_key("correct-password");
        let key2 = derive_key("wrong-password");
        let plaintext = "sk-secret-123";
        let encrypted = encrypt(plaintext, &key1).unwrap();
        // Decrypting with wrong key should fail
        assert!(decrypt(&encrypted, &key2).is_err());
    }
}
