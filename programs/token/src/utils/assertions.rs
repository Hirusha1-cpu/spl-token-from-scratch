//! Assertion Helper Functions
//!
//! Common validation checks used across all processors.
//! These functions make security checks consistent and readable.
//!
//! # Usage Pattern
//!
//! ```ignore
//! pub fn process(...) -> ProgramResult {
//!     // Validate everything first
//!     assert_owned_by(account, program_id)?;
//!     assert_signer(authority)?;
//!     assert_writable(account)?;
//!     
//!     // Then do the actual work
//!     ...
//! }
//! ```

use crate::error::TokenError;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
};

// =============================================================================
// OWNERSHIP CHECKS
// =============================================================================

/// Assert that an account is owned by the expected program.
///
/// # Why This Matters
///
/// An attacker could create a fake account with malicious data
/// owned by their own program. Without this check, we might
/// accept it as a real Mint or Token Account.
///
/// # Arguments
///
/// * `account` - The account to check
/// * `owner` - Expected owner (usually our program_id)
///
/// # Errors
///
/// Returns `InvalidAccountOwner` if the owner doesn't match.
///
/// # Example
///
/// ```ignore
/// // Ensure mint is owned by our token program
/// assert_owned_by(mint_info, program_id)?;
/// ```
pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(TokenError::InvalidAccountOwner.into())
    } else {
        Ok(())
    }
}

// =============================================================================
// SIGNER CHECKS
// =============================================================================

/// Assert that an account is a signer of the transaction.
///
/// # Why This Matters
///
/// If we don't check that the authority signed, anyone could
/// pretend to be the authority and steal tokens.
///
/// # Arguments
///
/// * `account` - The account to check
///
/// # Errors
///
/// Returns `MissingRequiredSignature` if not a signer.
///
/// # Example
///
/// ```ignore
/// // Ensure the owner actually signed this transaction
/// assert_signer(owner_info)?;
/// ```
pub fn assert_signer(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer {
        Err(ProgramError::MissingRequiredSignature)
    } else {
        Ok(())
    }
}

// =============================================================================
// WRITABLE CHECKS
// =============================================================================

/// Assert that an account is writable.
///
/// # Why This Matters
///
/// If an account isn't marked writable in the transaction,
/// the runtime will reject any modifications to it.
/// This check gives a clearer error earlier.
///
/// # Arguments
///
/// * `account` - The account to check
///
/// # Errors
///
/// Returns `InvalidAccountData` if not writable.
///
/// # Example
///
/// ```ignore
/// // Ensure we can modify the token account
/// assert_writable(token_account_info)?;
/// ```
pub fn assert_writable(account: &AccountInfo) -> ProgramResult {
    if !account.is_writable {
        Err(ProgramError::InvalidAccountData)
    } else {
        Ok(())
    }
}

// =============================================================================
// SIZE CHECKS
// =============================================================================

/// Assert that an account has the expected data length.
///
/// # Why This Matters
///
/// If the account is the wrong size, unpacking will fail or
/// read garbage data. This gives a clearer error message.
///
/// # Arguments
///
/// * `account` - The account to check
/// * `expected` - Expected data length in bytes
///
/// # Errors
///
/// Returns `InvalidAccountDataLength` if length doesn't match.
///
/// # Example
///
/// ```ignore
/// // Ensure account is exactly Mint::LEN bytes
/// assert_data_length(mint_info, Mint::LEN)?;
/// ```
pub fn assert_data_length(account: &AccountInfo, expected: usize) -> ProgramResult {
    if account.data_len() != expected {
        Err(TokenError::InvalidAccountDataLength.into())
    } else {
        Ok(())
    }
}

// =============================================================================
// RENT CHECKS
// =============================================================================

/// Assert that an account is rent exempt.
///
/// # Why This Matters
///
/// Accounts that aren't rent-exempt will be garbage collected
/// by the runtime, causing loss of funds or data.
///
/// # Arguments
///
/// * `rent` - The Rent sysvar
/// * `account` - The account to check
///
/// # Errors
///
/// Returns `NotRentExempt` if the account doesn't have enough lamports.
///
/// # Example
///
/// ```ignore
/// let rent = Rent::from_account_info(rent_info)?;
/// assert_rent_exempt(&rent, mint_info)?;
/// ```
pub fn assert_rent_exempt(rent: &Rent, account: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account.lamports(), account.data_len()) {
        Err(TokenError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

// =============================================================================
// CHECKED ARITHMETIC
// =============================================================================

/// Checked addition that returns a clear error on overflow.
///
/// # Why This Matters
///
/// Without checked arithmetic, overflow wraps around:
/// - u64::MAX + 1 = 0
/// - This could allow minting infinite tokens
///
/// # Arguments
///
/// * `a` - First operand
/// * `b` - Second operand
///
/// # Returns
///
/// * `Ok(a + b)` - If no overflow
/// * `Err(Overflow)` - If overflow would occur
///
/// # Example
///
/// ```ignore
/// // Safe: will error if overflow
/// mint.supply = checked_add(mint.supply, amount)?;
/// account.amount = checked_add(account.amount, amount)?;
/// ```
pub fn checked_add(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_add(b).ok_or_else(|| TokenError::Overflow.into())
}

/// Checked subtraction that returns a clear error on underflow.
///
/// # Why This Matters
///
/// Without checked arithmetic, underflow wraps around:
/// - 0 - 1 = u64::MAX
/// - This could allow spending tokens you don't have
///
/// # Arguments
///
/// * `a` - First operand (minuend)
/// * `b` - Second operand (subtrahend)
///
/// # Returns
///
/// * `Ok(a - b)` - If no underflow
/// * `Err(InsufficientFunds)` - If underflow would occur
///
/// # Example
///
/// ```ignore
/// // Safe: will error if insufficient funds
/// source.amount = checked_sub(source.amount, amount)?;
/// mint.supply = checked_sub(mint.supply, amount)?;
/// ```
pub fn checked_sub(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_sub(b)
        .ok_or_else(|| TokenError::InsufficientFunds.into())
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checked_add_success() {
        assert_eq!(checked_add(100, 200).unwrap(), 300);
        assert_eq!(checked_add(0, 0).unwrap(), 0);
        assert_eq!(checked_add(u64::MAX - 1, 1).unwrap(), u64::MAX);
    }

    #[test]
    fn test_checked_add_overflow() {
        assert!(checked_add(u64::MAX, 1).is_err());
        assert!(checked_add(u64::MAX, u64::MAX).is_err());
    }

    #[test]
    fn test_checked_sub_success() {
        assert_eq!(checked_sub(300, 200).unwrap(), 100);
        assert_eq!(checked_sub(100, 100).unwrap(), 0);
        assert_eq!(checked_sub(u64::MAX, u64::MAX).unwrap(), 0);
    }

    #[test]
    fn test_checked_sub_underflow() {
        assert!(checked_sub(0, 1).is_err());
        assert!(checked_sub(100, 101).is_err());
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHY ASSERTIONS?
===============

Every processor needs the same checks:
1. Is this account owned by our program?
2. Did the authority sign?
3. Is the account writable?
4. Is it the right size?
5. Is it rent exempt?

Without helper functions:
- Copy-paste the same checks everywhere
- Easy to forget a check
- Inconsistent error messages

With helper functions:
- One line per check
- Easy to audit
- Consistent errors

SECURITY IMPLICATIONS
=====================

1. assert_owned_by
   
   Attack without it:
   - Attacker creates account with fake Mint data
   - Owned by attacker's program
   - Our program reads it as valid
   - Attacker mints infinite tokens
   
   With the check:
   - We only trust accounts we own
   - Attacker can't inject fake data

2. assert_signer
   
   Attack without it:
   - Attacker includes someone else's pubkey
   - Claims to be that authority
   - Steals their tokens
   
   With the check:
   - Authority must have actually signed
   - Only real owner can authorize

3. checked_add / checked_sub
   
   Attack without it:
   - Overflow: mint.supply = MAX + 1 = 0
   - Underflow: balance = 0 - 1 = MAX
   - Infinite tokens!
   
   With the check:
   - Operation fails safely
   - No wrap-around

THE PATTERN
===========

Every processor follows this pattern:

```rust
pub fn process(...) -> ProgramResult {
    // 1. Parse accounts
    let account_iter = &mut accounts.iter();
    let account1 = next_account_info(account_iter)?;
    let account2 = next_account_info(account_iter)?;
    
    // 2. Validate EVERYTHING
    assert_owned_by(account1, program_id)?;
    assert_writable(account1)?;
    assert_signer(authority)?;
    // ... more checks ...
    
    // 3. Unpack state
    let mut state = State::unpack(&account1.data.borrow())?;
    
    // 4. More validation
    if !state.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }
    
    // 5. Update state (with checked math!)
    state.amount = checked_add(state.amount, amount)?;
    
    // 6. Pack state back
    state.pack_into_slice(&mut account1.data.borrow_mut())?;
    
    Ok(())
}
    */