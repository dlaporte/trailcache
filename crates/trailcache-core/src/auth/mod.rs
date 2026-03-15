//! Authentication module for managing user sessions and credentials.
//!
//! This module provides:
//! - `Session`: Token-based session management with automatic expiry
//! - `CredentialStore`: Secure OS-level credential storage via keyring
//!
//! Sessions are persisted to disk and tokens expire after 30 minutes.

pub mod credentials;
pub mod session;

pub use credentials::CredentialStore;
pub use session::{Session, SessionData};
