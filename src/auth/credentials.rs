// Allow dead code: Infrastructure methods for future use
#![allow(dead_code)]

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "scoutbook-tui";

pub struct CredentialStore;

impl CredentialStore {
    /// Store username and password in the OS keychain
    pub fn store(username: &str, password: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, username)
            .context("Failed to create keyring entry")?;
        entry
            .set_password(password)
            .context("Failed to store password in keychain")?;
        Ok(())
    }

    /// Retrieve password for a username from the OS keychain
    pub fn get_password(username: &str) -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, username)
            .context("Failed to create keyring entry")?;
        entry
            .get_password()
            .context("Failed to retrieve password from keychain")
    }

    /// Delete stored credentials for a username
    pub fn delete(username: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, username)
            .context("Failed to create keyring entry")?;
        entry
            .delete_credential()
            .context("Failed to delete credential from keychain")?;
        Ok(())
    }

    /// Check if credentials exist for a username
    pub fn has_credentials(username: &str) -> bool {
        if let Ok(entry) = Entry::new(SERVICE_NAME, username) {
            entry.get_password().is_ok()
        } else {
            false
        }
    }
}
