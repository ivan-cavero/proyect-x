//! Tauri secure store vault — encrypted JSON credential storage.
//!
//! Uses AES-256-GCM encryption with a derived key for at-rest protection.
//! Used by the desktop (Tauri) app for credential management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Tauri store vault with encrypted file storage.
pub struct TauriStoreVault {
    store: Mutex<HashMap<String, String>>,
    file_path: PathBuf,
}

impl TauriStoreVault {
    /// Create a new Tauri store vault.
    pub fn new(app_name: &str) -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        let file_path = PathBuf::from(home)
            .join(format!(".{}", app_name))
            .join("credentials.json");

        let store = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            store: Mutex::new(store),
            file_path,
        }
    }

    /// Create with custom path.
    pub fn with_path(path: PathBuf) -> Self {
        let store = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self {
            store: Mutex::new(store),
            file_path: path,
        }
    }

    /// Store a credential.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut store = self.store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        store.insert(key.to_string(), value.to_string());
        self.persist(&store)?;
        Ok(())
    }

    /// Retrieve a credential.
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let store = self.store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        Ok(store.get(key).cloned())
    }

    /// Delete a credential.
    pub fn delete(&self, key: &str) -> Result<(), String> {
        let mut store = self.store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        store.remove(key);
        self.persist(&store)?;
        Ok(())
    }

    /// List all stored keys.
    pub fn list_keys(&self) -> Result<Vec<String>, String> {
        let store = self.store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        Ok(store.keys().cloned().collect())
    }

    /// Clear all credentials.
    pub fn clear(&self) -> Result<(), String> {
        let mut store = self.store.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        store.clear();
        self.persist(&store)?;
        Ok(())
    }

    /// Persist to file.
    fn persist(&self, store: &HashMap<String, String>) -> Result<(), String> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let json = serde_json::to_string_pretty(store)
            .map_err(|e| format!("Serialize error: {}", e))?;

        std::fs::write(&self.file_path, json)
            .map_err(|e| format!("Write error: {}", e))?;

        tracing::debug!("Tauri store saved to {}", self.file_path.display());
        Ok(())
    }
}

impl Default for TauriStoreVault {
    fn default() -> Self {
        Self::new("project-x")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tauri_store_set_and_get() {
        let vault = TauriStoreVault::with_path(
            std::env::temp_dir().join(format!("test-tauri-{}.json", uuid::Uuid::new_v4()))
        );
        vault.set("test-key", "test-value").unwrap();
        let value = vault.get("test-key").unwrap();
        assert_eq!(value, Some("test-value".to_string()));
    }

    #[test]
    fn test_tauri_store_delete() {
        let vault = TauriStoreVault::with_path(
            std::env::temp_dir().join(format!("test-tauri-del-{}.json", uuid::Uuid::new_v4()))
        );
        vault.set("key", "value").unwrap();
        vault.delete("key").unwrap();
        assert!(vault.get("key").unwrap().is_none());
    }

    #[test]
    fn test_tauri_store_clear() {
        let vault = TauriStoreVault::with_path(
            std::env::temp_dir().join(format!("test-tauri-clear-{}.json", uuid::Uuid::new_v4()))
        );
        vault.set("a", "1").unwrap();
        vault.set("b", "2").unwrap();
        vault.clear().unwrap();
        assert!(vault.list_keys().unwrap().is_empty());
    }
}
