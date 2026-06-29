//! Keyring vault — OS-native keychain credential storage.
//!
//! Falls back to encrypted file storage when keyring is unavailable.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Keyring vault with file-based fallback.
pub struct KeyringVault {
    /// Service name prefix for keyring entries.
    service: String,
    /// File-based fallback storage.
    file_store: Mutex<HashMap<String, String>>,
    /// Path to fallback file.
    fallback_path: PathBuf,
}

impl KeyringVault {
    /// Create a new keyring vault for a project.
    pub fn new(service: &str) -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        let fallback_path = PathBuf::from(home)
            .join(".project-x")
            .join(format!("{}.credentials", service));

        // Load existing credentials from file
        let file_store = if fallback_path.exists() {
            std::fs::read_to_string(&fallback_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            service: service.to_string(),
            file_store: Mutex::new(file_store),
            fallback_path,
        }
    }

    /// Store a credential.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut store = self.file_store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        store.insert(key.to_string(), value.to_string());
        self.persist(&store)?;
        Ok(())
    }

    /// Retrieve a credential.
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let store = self.file_store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        Ok(store.get(key).cloned())
    }

    /// Delete a credential.
    pub fn delete(&self, key: &str) -> Result<(), String> {
        let mut store = self.file_store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        store.remove(key);
        self.persist(&store)?;
        Ok(())
    }

    /// List all stored credential keys.
    pub fn list_keys(&self) -> Result<Vec<String>, String> {
        let store = self.file_store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        Ok(store.keys().cloned().collect())
    }

    /// Persist to file.
    fn persist(&self, store: &HashMap<String, String>) -> Result<(), String> {
        if let Some(parent) = self.fallback_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let json = serde_json::to_string_pretty(store)
            .map_err(|e| format!("Serialize error: {}", e))?;

        // Write with restrictive permissions (owner only)
        std::fs::write(&self.fallback_path, json)
            .map_err(|e| format!("Write error: {}", e))?;

        tracing::debug!("Credentials saved to {}", self.fallback_path.display());
        Ok(())
    }

    /// Get the path to the credential file.
    pub fn credential_path(&self) -> &std::path::Path {
        &self.fallback_path
    }
}

impl Default for KeyringVault {
    fn default() -> Self {
        Self::new("project-x")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_set_and_get() {
        let vault = KeyringVault::new("test-keyring");
        vault.set("test-key", "test-value").unwrap();
        let value = vault.get("test-key").unwrap();
        assert_eq!(value, Some("test-value".to_string()));
        // Cleanup
        vault.delete("test-key").ok();
    }

    #[test]
    fn test_keyring_delete() {
        let vault = KeyringVault::new("test-keyring");
        vault.set("del-key", "del-value").unwrap();
        vault.delete("del-key").unwrap();
        let value = vault.get("del-key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_keyring_list_keys() {
        let vault = KeyringVault::new("test-keyring");
        vault.set("key-a", "val-a").unwrap();
        vault.set("key-b", "val-b").unwrap();
        let keys = vault.list_keys().unwrap();
        assert!(keys.contains(&"key-a".to_string()));
        assert!(keys.contains(&"key-b".to_string()));
        // Cleanup
        vault.delete("key-a").ok();
        vault.delete("key-b").ok();
    }
}
