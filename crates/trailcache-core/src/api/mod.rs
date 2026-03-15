//! REST API client module for Scouting.org services.
//!
//! This module provides the `ApiClient` for communicating with the
//! Scouting.org API to fetch roster, event, and advancement data.
//!
//! The API uses JWT bearer token authentication obtained through
//! the my.scouting.org authentication endpoint.

pub mod client;
pub mod error;

pub use client::ApiClient;
pub use error::ApiError;
