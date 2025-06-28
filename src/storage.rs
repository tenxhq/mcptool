use keyring::{Entry, Error as KeyringError};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

const SERVICE_NAME: &str = "mcptool";
const KEYRING_PREFIX: &str = "oauth_";

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Keyring error: {0}")]
    Keyring(#[from] KeyringError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Failed to get config directory")]
    ConfigDir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuth {
    pub name: String,
    pub server_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub access_token: Option<SecretString>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<SystemTime>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthMetadata {
    pub name: String,
    pub server_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<SystemTime>,
}

pub struct TokenStorage {
    config_dir: PathBuf,
}

impl TokenStorage {
    pub fn new() -> Result<Self, StorageError> {
        let config_dir = dirs::config_dir()
            .ok_or(StorageError::ConfigDir)?
            .join("mcptool");

        fs::create_dir_all(&config_dir)?;

        Ok(Self { config_dir })
    }

    pub fn store_auth(&self, auth: &StoredAuth) -> Result<(), StorageError> {
        // Store metadata in config file
        let metadata = AuthMetadata {
            name: auth.name.clone(),
            server_url: auth.server_url.clone(),
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret.clone(),
            auth_url: auth.auth_url.clone(),
            token_url: auth.token_url.clone(),
            redirect_url: auth.redirect_url.clone(),
            scopes: auth.scopes.clone(),
            expires_at: auth.expires_at,
        };

        let metadata_path = self.config_dir.join("auth.json");
        let mut all_metadata = self.load_all_metadata()?;
        all_metadata.insert(auth.name.clone(), metadata);

        let json = serde_json::to_string_pretty(&all_metadata)?;
        fs::write(metadata_path, json)?;

        // Store access token in keyring
        if let Some(access_token) = &auth.access_token {
            let entry = Entry::new(
                SERVICE_NAME,
                &format!("{}{}_access", KEYRING_PREFIX, auth.name),
            )?;
            entry.set_password(access_token.expose_secret())?;
        }

        // Store refresh token in keyring
        if let Some(refresh_token) = &auth.refresh_token {
            let entry = Entry::new(
                SERVICE_NAME,
                &format!("{}{}_refresh", KEYRING_PREFIX, auth.name),
            )?;
            entry.set_password(refresh_token)?;
        }

        Ok(())
    }

    pub fn get_auth(&self, name: &str) -> Result<StoredAuth, StorageError> {
        let all_metadata = self.load_all_metadata()?;
        let metadata = all_metadata
            .get(name)
            .ok_or_else(|| StorageError::NotFound(name.to_string()))?;

        // Try to load access token from keyring
        let access_token =
            match Entry::new(SERVICE_NAME, &format!("{}{}_access", KEYRING_PREFIX, name)) {
                Ok(entry) => match entry.get_password() {
                    Ok(token) => Some(SecretString::new(token.into())),
                    Err(KeyringError::NoEntry) => None,
                    Err(e) => return Err(e.into()),
                },
                Err(_) => None,
            };

        // Try to load refresh token from keyring
        let refresh_token =
            match Entry::new(SERVICE_NAME, &format!("{}{}_refresh", KEYRING_PREFIX, name)) {
                Ok(entry) => match entry.get_password() {
                    Ok(token) => Some(token),
                    Err(KeyringError::NoEntry) => None,
                    Err(e) => return Err(e.into()),
                },
                Err(_) => None,
            };

        Ok(StoredAuth {
            name: metadata.name.clone(),
            server_url: metadata.server_url.clone(),
            client_id: metadata.client_id.clone(),
            client_secret: metadata.client_secret.clone(),
            access_token,
            refresh_token,
            expires_at: metadata.expires_at,
            auth_url: metadata.auth_url.clone(),
            token_url: metadata.token_url.clone(),
            redirect_url: metadata.redirect_url.clone(),
            scopes: metadata.scopes.clone(),
        })
    }

    pub fn remove_auth(&self, name: &str) -> Result<(), StorageError> {
        // Remove from metadata
        let metadata_path = self.config_dir.join("auth.json");
        let mut all_metadata = self.load_all_metadata()?;

        if !all_metadata.contains_key(name) {
            return Err(StorageError::NotFound(name.to_string()));
        }

        all_metadata.remove(name);
        let json = serde_json::to_string_pretty(&all_metadata)?;
        fs::write(metadata_path, json)?;

        // Try to remove tokens from keyring (ignore errors if not found)
        if let Ok(entry) = Entry::new(SERVICE_NAME, &format!("{}{}_access", KEYRING_PREFIX, name)) {
            let _ = entry.delete_credential();
        }

        if let Ok(entry) = Entry::new(SERVICE_NAME, &format!("{}{}_refresh", KEYRING_PREFIX, name))
        {
            let _ = entry.delete_credential();
        }

        Ok(())
    }

    pub fn list_auth(&self) -> Result<Vec<String>, StorageError> {
        let all_metadata = self.load_all_metadata()?;
        let mut names: Vec<String> = all_metadata.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    pub fn get_all_auth(&self) -> Result<Vec<StoredAuth>, StorageError> {
        let names = self.list_auth()?;
        let mut auths = Vec::new();

        for name in names {
            match self.get_auth(&name) {
                Ok(auth) => auths.push(auth),
                Err(_) => continue, // Skip entries that can't be loaded
            }
        }

        Ok(auths)
    }

    fn load_all_metadata(&self) -> Result<HashMap<String, AuthMetadata>, StorageError> {
        let metadata_path = self.config_dir.join("auth.json");

        if !metadata_path.exists() {
            return Ok(HashMap::new());
        }

        let contents = fs::read_to_string(metadata_path)?;
        let metadata: HashMap<String, AuthMetadata> = serde_json::from_str(&contents)?;
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_lifecycle() {
        let storage = TokenStorage::new().expect("Failed to create storage");

        // Create test auth
        let auth = StoredAuth {
            name: "test_auth".to_string(),
            server_url: "https://example.com".to_string(),
            client_id: "test_client".to_string(),
            client_secret: Some("test_secret".to_string()),
            access_token: Some(SecretString::new("test_token".to_string().into())),
            refresh_token: Some("test_refresh".to_string()),
            expires_at: Some(SystemTime::now()),
            auth_url: "https://example.com/auth".to_string(),
            token_url: "https://example.com/token".to_string(),
            redirect_url: Some("http://localhost:8080".to_string()),
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        // Store
        storage.store_auth(&auth).expect("Failed to store auth");

        // List
        let names = storage.list_auth().expect("Failed to list auth");
        assert!(names.contains(&"test_auth".to_string()));

        // Get
        let retrieved = storage.get_auth("test_auth").expect("Failed to get auth");
        assert_eq!(retrieved.name, auth.name);
        assert_eq!(retrieved.server_url, auth.server_url);

        // Remove
        storage
            .remove_auth("test_auth")
            .expect("Failed to remove auth");

        // Verify removed
        let names = storage.list_auth().expect("Failed to list auth");
        assert!(!names.contains(&"test_auth".to_string()));
    }
}
