//! API Handlers
//!
//! Request handlers for all API endpoints.
//! Each module handles a specific domain.

pub mod health;
pub mod auth;
pub mod account;
pub mod wallet;
pub mod order;
pub mod market;

pub use health::*;
