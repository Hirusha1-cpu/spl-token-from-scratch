//! # SPL Token From Scratch
//!
//! A complete implementation of the SPL Token program built from scratch.
//! This is an educational implementation demonstrating how token programs work.
//!
//! ## Overview
//!
//! This program allows you to:
//! - Create token mints (define new token types)
//! - Create token accounts (wallets for specific tokens)
//! - Mint new tokens (increase supply)
//! - Transfer tokens between accounts
//! - Burn tokens (decrease supply)
//! - Approve delegates (allow others to spend your tokens)
//! - Close accounts (reclaim rent)
//!
//! ## Account Types
//!
//! | Account Type | Size | Description |
//! |--------------|------|-------------|
//! | Mint | 82 bytes | Defines a token type |
//! | Account | 165 bytes | Holds tokens for an owner |
//! | Multisig | 355 bytes | M-of-N authority |
//!
//! ## Instructions
//!
//! | # | Instruction | Description |
//! |---|-------------|-------------|
//! | 0 | InitializeMint | Create a new token mint |
//! | 1 | InitializeAccount | Create a new token account |
//! | 2 | InitializeMultisig | Create a multisig authority |
//! | 3 | Transfer | Transfer tokens |
//! | 4 | Approve | Approve a delegate |
//! | 5 | Revoke | Revoke a delegate |
//! | 6 | SetAuthority | Change an authority |
//! | 7 | MintTo | Mint new tokens |
//! | 8 | Burn | Burn tokens |
//! | 9 | CloseAccount | Close and reclaim rent |
//! | 10 | FreezeAccount | Freeze an account |
//! | 11 | ThawAccount | Thaw a frozen account |

// =============================================================================
// MODULE DECLARATIONS
// =============================================================================

/// Program entrypoint - where Solana calls into our program
pub mod entrypoint;

/// Custom error types with unique codes
pub mod error;

/// Instruction definitions and parsing
pub mod instruction;

/// Instruction processors (business logic)
pub mod processor;

/// Account state structures (Mint, Account, Multisig)
pub mod state;

/// Utility functions for validation and math
pub mod utils;

// =============================================================================
// RE-EXPORTS
// =============================================================================

// Make commonly used types available at crate root
// Users can write: use spl_token_from_scratch::TokenError;
// Instead of: use spl_token_from_scratch::error::TokenError;

pub use error::TokenError;
pub use instruction::{AuthorityType, TokenInstruction};
pub use processor::Processor;
pub use state::{Account, AccountState, Mint, Multisig, Pack};

// =============================================================================
// PROGRAM ID
// =============================================================================

// This macro declares the program's on-chain address
// Replace with your actual program ID after deployment
solana_program::declare_id!("TokenFromScratch111111111111111111111111111");

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHAT IS lib.rs?
===============
lib.rs is the "root" of a Rust library crate.
It's like index.js in JavaScript or __init__.py in Python.

Everything your crate exposes starts here:
- Module declarations (pub mod xxx)
- Re-exports (pub use xxx)
- Crate-level documentation

MODULE SYSTEM
=============

pub mod entrypoint;
    ↓
Rust looks for:
1. entrypoint.rs (file in same directory)
2. entrypoint/mod.rs (directory with mod.rs)

We use option 1 for simple modules, option 2 for complex ones.

Our structure:
src/
├── lib.rs           <- You are here
├── entrypoint.rs    <- pub mod entrypoint
├── error.rs         <- pub mod error
├── instruction.rs   <- pub mod instruction
├── processor/       <- pub mod processor (uses mod.rs)
│   └── mod.rs
├── state/           <- pub mod state (uses mod.rs)
│   └── mod.rs
└── utils/           <- pub mod utils (uses mod.rs)
    └── mod.rs

DECLARE_ID MACRO
================

solana_program::declare_id!("TokenFromScratch11111111111111111111111111");

This creates:
- A constant `ID` of type Pubkey
- A function `id()` that returns the Pubkey
- A function `check_id(id: &Pubkey) -> bool`

The string is a base58-encoded 32-byte public key.

After deployment, you'll update this with your actual program ID:
1. Deploy: solana program deploy target/deploy/spl_token_from_scratch.so
2. Copy the program ID from output
3. Paste it in declare_id!()
4. Rebuild and redeploy (or just update for clients)

WHY RE-EXPORTS?
===============

Without re-exports:
    use spl_token_from_scratch::error::TokenError;
    use spl_token_from_scratch::instruction::TokenInstruction;
    use spl_token_from_scratch::state::Mint;

With re-exports:
    use spl_token_from_scratch::{TokenError, TokenInstruction, Mint};

Much cleaner! Common pattern in Rust libraries.

DOCUMENTATION COMMENTS
======================

//! at file start = Module/crate documentation
/// before items = Item documentation

These become HTML docs with `cargo doc --open`.
Write them for yourself and others!
*/