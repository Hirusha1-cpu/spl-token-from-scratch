//! Utility Modules
//!
//! This module provides helper functions used across all processors.
//!
//! # Modules
//!
//! - `assertions`: Common validation checks (ownership, signer, etc.)
//! - `authority`: Authority validation (single signer and multisig)

pub mod assertions;
pub mod authority;

// Re-export all utilities for easy access
pub use assertions::*;
pub use authority::*;