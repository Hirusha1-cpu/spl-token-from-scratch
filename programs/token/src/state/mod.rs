//! Account State Structures
//!
//! This module defines the data structures stored in Solana accounts.
//!
//! # Account Types
//!
//! | Type | Size | Description |
//! |------|------|-------------|
//! | Mint | 82 bytes | Defines a token type |
//! | Account | 165 bytes | Holds tokens for an owner |
//! | Multisig | 355 bytes | M-of-N multisig authority |
//!
//! # Serialization
//!
//! All structures use fixed-size, deterministic serialization:
//! - Little-endian for integers
//! - No padding between fields
//! - Same data always produces same bytes
//!
//! # The Pack Trait
//!
//! All state types implement the `Pack` trait for serialization:
//!
//! ```ignore
//! let mint = Mint::unpack(&account.data.borrow())?;  // Read
//! mint.pack(&mut account.data.borrow_mut())?;        // Write
//! ```

// =============================================================================
// SUBMODULES
// =============================================================================

pub mod account;
pub mod mint;
pub mod multisig;

// =============================================================================
// RE-EXPORTS
// =============================================================================

pub use account::{Account, AccountState};
pub use mint::Mint;
pub use multisig::{Multisig, MAX_SIGNERS};

use solana_program::program_error::ProgramError;

// =============================================================================
// PACK TRAIT
// =============================================================================

/// Trait for packing/unpacking account state to/from bytes.
///
/// All state structures must implement this trait.
/// It provides a consistent interface for serialization.
///
/// # Why Not Borsh?
///
/// We use manual serialization because:
/// 1. SPL Token compatibility requires exact byte layouts
/// 2. Fixed sizes are enforced at compile time
/// 3. No serialization overhead
/// 4. Full control over the format
///
/// # Example Implementation
///
/// ```ignore
/// impl Pack for MyState {
///     const LEN: usize = 40;
///
///     fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
///         let input = array_ref![input, 0, Self::LEN];
///         // Parse fields...
///         Ok(MyState { ... })
///     }
///
///     fn pack(&self, output: &mut [u8]) -> Result<(), ProgramError> {
///         let output = array_mut_ref![output, 0, Self::LEN];
///         // Write fields...
///         Ok(())
///     }
/// }
/// ```
pub trait Pack: Sized {
    /// The fixed size in bytes when serialized.
    ///
    /// This is used to:
    /// - Validate account data length
    /// - Allocate accounts with correct size
    /// - Calculate rent exemption
    const LEN: usize;

    /// Deserialize from a byte slice.
    ///
    /// # Arguments
    /// * `input` - Byte slice containing serialized data
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully deserialized
    /// * `Err(...)` - Data is invalid
    ///
    /// # Panics
    /// May panic if input.len() < Self::LEN (use unpack_from_slice instead)
    fn unpack(input: &[u8]) -> Result<Self, ProgramError>;

    /// Serialize into a byte slice.
    ///
    /// # Arguments
    /// * `output` - Mutable byte slice to write into
    ///
    /// # Returns
    /// * `Ok(())` - Successfully serialized
    /// * `Err(...)` - Output is wrong size
    fn pack(&self, output: &mut [u8]) -> Result<(), ProgramError>;

    /// Unpack with length validation.
    ///
    /// Checks that `src.len() == Self::LEN` before unpacking.
    /// Use this instead of `unpack` when you have untrusted input.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let account = &account_info.data.borrow();
    /// let mint = Mint::unpack_from_slice(account)?;
    /// ```
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Self::unpack(src)
    }

    /// Pack with length validation.
    ///
    /// Checks that `dst.len() == Self::LEN` before packing.
    /// Use this instead of `pack` for safety.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let account = &mut account_info.data.borrow_mut();
    /// mint.pack_into_slice(account)?;
    /// ```
    fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        self.pack(dst)
    }
}

// =============================================================================
// COPTION - COMPACT OPTIONAL TYPE
// =============================================================================

/// A compact optional type for on-chain storage.
///
/// # Why Not std::Option?
///
/// Rust's `Option<T>` doesn't have a stable memory layout.
/// For on-chain storage, we need:
/// - Deterministic serialization
/// - Known byte layout
/// - Compatibility with SPL Token
///
/// # Layout
///
/// ```text
/// COption<Pubkey>: 36 bytes
/// [tag: 4 bytes, little-endian u32][value: 32 bytes]
///
/// Tag = 0: None (value bytes are zeros)
/// Tag = 1: Some (value bytes contain the Pubkey)
/// ```
///
/// # Example
///
/// ```ignore
/// let authority: COption<Pubkey> = COption::some(my_pubkey);
/// if authority.is_some() {
///     let pubkey = authority.unwrap();
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct COption<T> {
    /// The underlying Option value
    value: Option<T>,
}

impl<T> COption<T> {
    /// Create a COption with a value (Some variant).
    ///
    /// # Example
    /// ```ignore
    /// let authority = COption::some(my_pubkey);
    /// ```
    pub fn some(value: T) -> Self {
        Self { value: Some(value) }
    }

    /// Create an empty COption (None variant).
    ///
    /// # Example
    /// ```ignore
    /// let no_authority: COption<Pubkey> = COption::none();
    /// ```
    pub fn none() -> Self {
        Self { value: None }
    }

    /// Check if the COption contains a value.
    pub fn is_some(&self) -> bool {
        self.value.is_some()
    }

    /// Check if the COption is empty.
    pub fn is_none(&self) -> bool {
        self.value.is_none()
    }

    /// Get a reference to the inner value, if present.
    ///
    /// Returns `Some(&T)` if value exists, `None` otherwise.
    pub fn as_ref(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Get a mutable reference to the inner value, if present.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }

    /// Unwrap the value, panicking if empty.
    ///
    /// # Panics
    /// Panics if the COption is None.
    pub fn unwrap(self) -> T {
        self.value.unwrap()
    }

    /// Unwrap the value or return a default.
    pub fn unwrap_or(self, default: T) -> T {
        self.value.unwrap_or(default)
    }

    /// Map the inner value using a function.
    ///
    /// # Example
    /// ```ignore
    /// let opt: COption<u64> = COption::some(5);
    /// let doubled: COption<u64> = opt.map(|x| x * 2);
    /// ```
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> COption<U> {
        COption {
            value: self.value.map(f),
        }
    }
}

// Allow conversion from standard Option
impl<T> From<Option<T>> for COption<T> {
    fn from(opt: Option<T>) -> Self {
        Self { value: opt }
    }
}

// Allow conversion to standard Option
impl<T> From<COption<T>> for Option<T> {
    fn from(copt: COption<T>) -> Self {
        copt.value
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHY FIXED SIZES MATTER
======================

Solana accounts have fixed sizes set at creation.
You cannot resize an account (except with realloc, which has limits).

When creating a Mint account:
1. Calculate size: Mint::LEN = 82 bytes
2. Calculate rent: rent.minimum_balance(82)
3. Create account with that size

If your struct changes size, old accounts become invalid!
That's why we use:
- Fixed-size arrays, not Vec
- COption instead of Option
- Explicit byte counts

DETERMINISTIC SERIALIZATION
===========================

For the same data, we must always produce the same bytes.

Why?
1. Account hashing (accounts are identified by their data hash)
2. Debugging (can compare bytes directly)
3. Cross-language compatibility (TypeScript must produce same bytes)

Non-deterministic would be:
- HashMap (iteration order varies)
- Floating point (platform differences)
- Padding bytes (random values)

THE COPTION TYPE
================

Standard Option<T> in Rust:
- Compiler can choose any layout
- Layout might change between Rust versions
- No guaranteed serialization

Our COption<T>:
- Fixed layout: [u32 tag][T value]
- Tag 0 = None, Tag 1 = Some
- Same as SPL Token's COption

Memory comparison:
    Option<Pubkey>:  32 or 33 bytes (depends on optimization)
    COption<Pubkey>: 36 bytes (always)

WHY 4-BYTE TAG?
===============

You might think: "A bool is 1 byte, why use 4?"

Reasons:
1. Alignment: 4-byte alignment is faster on most CPUs
2. Compatibility: Matches SPL Token exactly
3. Future-proofing: Room for additional states (2, 3, etc.)

PACK TRAIT DESIGN
=================

The Pack trait has two levels:

Basic (must implement):
- unpack(&[u8]) -> Result<Self>
- pack(&mut [u8]) -> Result<()>

Safe wrappers (provided):
- unpack_from_slice: Validates length first
- pack_into_slice: Validates length first

Always use the _slice variants in processors!
They catch size mismatches early.

THE FROM TRAIT
==============

impl From<Option<T>> for COption<T>
impl From<COption<T>> for Option<T>

These allow seamless conversion:

    let std_opt: Option<Pubkey> = Some(key);
    let c_opt: COption<Pubkey> = std_opt.into();
    let back: Option<Pubkey> = c_opt.into();

This is ergonomic for:
- Converting user input to storage format
- Using standard Option methods after conversion
*/