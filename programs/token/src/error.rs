//! Custom Error Types
//!
//! This module defines all errors that the token program can return.
//! Each error has a unique numeric code that clients can match against.
//!
//! # Error Code Ranges
//!
//! | Range | Category |
//! |-------|----------|
//! | 0-9 | Account validation errors |
//! | 10-19 | Authority errors |
//! | 20-29 | Operation errors |
//! | 30-39 | Delegate errors |
//! | 40-49 | Multisig errors |
//! | 50-59 | Close errors |
//!
//! # Usage
//!
//! ```ignore
//! use crate::error::TokenError;
//!
//! fn some_check() -> ProgramResult {
//!     if !valid {
//!         return Err(TokenError::InvalidAuthority.into());
//!     }
//!     Ok(())
//! }
//! ```

use solana_program::program_error::ProgramError;
use thiserror::Error;

// =============================================================================
// ERROR ENUM
// =============================================================================

/// Errors that may be returned by the Token program.
///
/// Each variant becomes a unique error code when converted to ProgramError.
/// The codes are assigned based on the order of variants (0, 1, 2, ...).
///
/// # Important
///
/// After deployment, NEVER reorder these variants!
/// Clients depend on stable error codes.
/// Always add new errors at the end.
#[derive(Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum TokenError {
    // =========================================================================
    // ACCOUNT VALIDATION ERRORS (0-9)
    // =========================================================================

    /// Error 0: Account is not owned by the token program.
    ///
    /// Every account we operate on must be owned by our program.
    /// This prevents attackers from passing fake accounts.
    ///
    /// # Example
    /// An attacker creates an account owned by their program
    /// with data that looks like a valid Mint.
    /// Without this check, we might accept it as real.
    #[error("Account not owned by token program")]
    InvalidAccountOwner,

    /// Error 1: Account data has wrong length.
    ///
    /// Mint must be 82 bytes, Account must be 165 bytes, etc.
    /// Wrong size indicates corruption or attack.
    #[error("Invalid account data length")]
    InvalidAccountDataLength,

    /// Error 2: Account is not rent exempt.
    ///
    /// Accounts must have enough lamports to be rent-exempt.
    /// Otherwise, they could be garbage collected by the runtime.
    #[error("Account is not rent exempt")]
    NotRentExempt,

    /// Error 3: Account is already initialized.
    ///
    /// Prevents double-initialization attacks.
    /// Once initialized, an account cannot be re-initialized.
    #[error("Account already initialized")]
    AlreadyInitialized,

    /// Error 4: Account is not initialized.
    ///
    /// Cannot perform operations on uninitialized accounts.
    /// Must call Initialize* instruction first.
    #[error("Account not initialized")]
    UninitializedAccount,

    // =========================================================================
    // AUTHORITY ERRORS (5-9)
    // =========================================================================

    /// Error 5: Invalid authority provided.
    ///
    /// The signer does not match the expected authority.
    /// E.g., trying to mint without being mint_authority.
    #[error("Invalid authority")]
    InvalidAuthority,

    /// Error 6: Owner does not match.
    ///
    /// The token account's owner field doesn't match the signer.
    #[error("Owner mismatch")]
    OwnerMismatch,

    /// Error 7: Mint authority is required but not set.
    ///
    /// Trying to mint tokens, but mint_authority is None.
    /// This happens when mint authority was permanently revoked.
    #[error("Mint authority required")]
    MintAuthorityRequired,

    /// Error 8: Account is frozen.
    ///
    /// Frozen accounts cannot transfer tokens.
    /// Must be thawed by freeze_authority first.
    #[error("Account is frozen")]
    AccountFrozen,

    /// Error 9: Freeze authority is required but not set.
    ///
    /// Trying to freeze/thaw, but freeze_authority is None.
    #[error("Freeze authority required")]
    FreezeAuthorityRequired,

    // =========================================================================
    // OPERATION ERRORS (10-14)
    // =========================================================================

    /// Error 10: Insufficient funds.
    ///
    /// Account doesn't have enough tokens for the operation.
    /// E.g., trying to transfer 100 tokens when balance is 50.
    #[error("Insufficient funds")]
    InsufficientFunds,

    /// Error 11: Arithmetic overflow.
    ///
    /// An arithmetic operation would overflow.
    /// E.g., minting would push supply above u64::MAX.
    #[error("Arithmetic overflow")]
    Overflow,

    /// Error 12: Mint mismatch.
    ///
    /// Token accounts must be for the same mint.
    /// E.g., can't transfer USDC to a BONK account.
    #[error("Mint mismatch")]
    MintMismatch,

    /// Error 13: Non-zero token balance.
    ///
    /// Cannot close an account that still has tokens.
    /// Must transfer or burn all tokens first.
    #[error("Account has non-zero balance")]
    NonZeroBalance,

    /// Error 14: Invalid instruction data.
    ///
    /// Could not parse the instruction data.
    /// Wrong format, missing bytes, invalid discriminant.
    #[error("Invalid instruction")]
    InvalidInstruction,

    // =========================================================================
    // DELEGATE ERRORS (15-16)
    // =========================================================================

    /// Error 15: No delegate set.
    ///
    /// Trying to use delegate authority, but none is approved.
    #[error("No delegate set on account")]
    NoDelegate,

    /// Error 16: Insufficient delegated amount.
    ///
    /// Delegate is trying to transfer more than approved.
    #[error("Insufficient delegated amount")]
    InsufficientDelegatedAmount,

    // =========================================================================
    // MULTISIG ERRORS (17-19)
    // =========================================================================

    /// Error 17: Not enough signers.
    ///
    /// Multisig requires M signatures, but fewer were provided.
    #[error("Not enough multisig signers")]
    NotEnoughSigners,

    /// Error 18: Invalid multisig configuration.
    ///
    /// M > N, or N > 11, or M == 0, etc.
    #[error("Invalid multisig configuration")]
    InvalidMultisigConfig,

    /// Error 19: Signer not in multisig.
    ///
    /// One of the signers is not a member of the multisig.
    #[error("Invalid multisig signer")]
    InvalidMultisigSigner,

    // =========================================================================
    // CLOSE ERRORS (20-22)
    // =========================================================================

    /// Error 20: Close authority mismatch.
    ///
    /// The signer is not the close authority.
    #[error("Close authority mismatch")]
    CloseAuthorityMismatch,

    /// Error 21: Native account has balance.
    ///
    /// For wrapped SOL accounts, lamport balance counts.
    #[error("Native account has balance")]
    NativeAccountHasBalance,

    /// Error 22: Cannot transfer to self.
    ///
    /// Source and destination are the same account.
    #[error("Self transfer not allowed")]
    SelfTransfer,
}

// =============================================================================
// CONVERSION TO PROGRAMERROR
// =============================================================================

/// Convert TokenError to ProgramError.
///
/// This implementation allows using the `?` operator with our errors.
///
/// # How It Works
///
/// ```ignore
/// // This works because of this impl:
/// fn example() -> ProgramResult {
///     return Err(TokenError::InvalidAuthority.into());
/// }
///
/// // Equivalent to:
/// fn example() -> ProgramResult {
///     return Err(ProgramError::Custom(5)); // 5 = InvalidAuthority
/// }
/// ```
///
/// # Error Codes
///
/// The error code is simply the enum variant's position (0-indexed).
/// InvalidAccountOwner = 0, InvalidAccountDataLength = 1, etc.
impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHY CUSTOM ERRORS?
==================

Solana provides generic ProgramError variants like:
- InvalidArgument
- InvalidAccountData
- InsufficientFunds

But these are vague. Custom errors provide:
1. Specific error codes for debugging
2. Clear error messages for users
3. Easier client-side error handling

THE THISERROR CRATE
===================

#[derive(Error)] from thiserror does magic:

Before (manual implementation):
    impl std::fmt::Display for TokenError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                TokenError::InvalidAccountOwner =>
                    write!(f, "Account not owned by token program"),
                // ... 20 more matches
            }
        }
    }

    impl std::error::Error for TokenError {}

After (with thiserror):
    #[derive(Error)]
    pub enum TokenError {
        #[error("Account not owned by token program")]
        InvalidAccountOwner,
    }

One line per error instead of 50+ lines of boilerplate!

ERROR CODE STABILITY
====================

CRITICAL: Never reorder variants after deployment!

Why?
- Clients match on numeric codes
- If InvalidAuthority was 5 and becomes 6
- Client code breaks silently

Safe changes:
- Add new variants at the end
- Change error messages (string only)

Unsafe changes:
- Reorder variants
- Remove variants
- Insert variants in the middle

HOW CLIENTS SEE ERRORS
======================

When your program returns Err(TokenError::InsufficientFunds.into()):

1. On-chain: Returns ProgramError::Custom(10)

2. In transaction logs:
   "Program failed with error: Custom(10)"

3. In Solana Explorer:
   "Error: Custom program error: 0xa"  (0xa = 10 in hex)

4. Client SDK:
   try {
       await sendTransaction(tx);
   } catch (e) {
       if (e.code === 10) {
           // Insufficient funds
       }
   }

THE FROM TRAIT
==============

impl From<TokenError> for ProgramError

This enables:

1. The .into() method:
   Err(TokenError::Overflow.into())

2. The ? operator (implicitly calls .into()):
   some_function()?  // If returns Err(TokenError), converts to ProgramError

3. Ergonomic error returns:
   fn process() -> ProgramResult {
       if bad {
           return Err(TokenError::InvalidAuthority.into());
       }
       Ok(())
   }

PROGRAMERROR::CUSTOM
====================

ProgramError is Solana's standard error type.
ProgramError::Custom(u32) is for program-specific errors.

The u32 value is our error code.
We use `e as u32` to get the discriminant value.

In Rust, enums start at 0:
- InvalidAccountOwner = 0
- InvalidAccountDataLength = 1
- NotRentExempt = 2
- etc.
*/