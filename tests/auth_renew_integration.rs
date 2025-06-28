use mcptool::auth::renew_command;
use mcptool::output::Output;
use mcptool::storage::{StoredAuth, TokenStorage};
use std::time::{Duration, SystemTime};

#[tokio::test]
async fn test_renew_missing_auth() {
    let output = Output::new();

    let result = renew_command("nonexistent".to_string(), output).await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Entry not found: nonexistent")
    );
}

#[tokio::test]
async fn test_renew_no_refresh_token() {
    let storage = TokenStorage::new().unwrap();

    // Create auth entry without refresh token
    let auth = StoredAuth {
        name: "test_no_refresh".to_string(),
        server_url: "https://example.com".to_string(),
        client_id: "test_client".to_string(),
        client_secret: Some("test_secret".to_string()),
        access_token: Some("test_token".to_string()),
        refresh_token: None, // No refresh token
        expires_at: Some(SystemTime::now() - Duration::from_secs(3600)), // Expired
        auth_url: "https://example.com/auth".to_string(),
        token_url: "https://example.com/token".to_string(),
        redirect_url: Some("http://localhost:8080".to_string()),
        scopes: vec!["read".to_string()],
    };

    storage.store_auth(&auth).unwrap();

    let output = Output::new();
    let result = renew_command("test_no_refresh".to_string(), output).await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No refresh token available")
    );

    // Clean up
    storage.remove_auth("test_no_refresh").unwrap();
}

// Note: Testing actual token refresh would require a mock OAuth server
// or test doubles for the OAuth2 client. The above tests cover the
// basic validation logic that can be tested without external dependencies.
