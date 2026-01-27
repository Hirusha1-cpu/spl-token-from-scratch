//! Authority Validation Utilities
//!
//! Handles validation for both single-signer and multisig authorities.
//!
//! # Authority Types
//!
//! 1. **Single Signer**: A regular pubkey that must sign the transaction
//! 2. **Multisig**: An M-of-N account requiring M signatures from N possible signers
//!
//! # How Multisig Detection Works
//!
//! We detect if an authority is a multisig by checking:
//! - Account data length == 355 (Multisig::LEN)
//! - Account owner == our program_id
//!
//! If both conditions are met, we validate as multisig.
//! Otherwise, we validate as a single signer.
//!
//! # Example Usage
//!
//! ```ignore
//! // Validate mint_authority (could be single key or multisig)
//! validate_authority(
//!     program_id,
//!     &mint.mint_authority.unwrap(),
//!     authority_info,
//!     &signer_accounts,
//! )?;
//!
//! // Validate owner OR delegate
//! let used_delegate = validate_owner_or_delegate(
//!     program_id,
//!     &account.owner,
//!     account.delegate.as_ref(),
//!     authority_info,
//!     &signer_accounts,
//! )?;
//! ```

use crate::error::TokenError;
use crate::state::{Multisig, Pack};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// =============================================================================
// MAIN AUTHORITY VALIDATION
// =============================================================================

/// Validate that an authority is correct and has authorized the action.
///
/// This function handles both single-signer and multisig authorities.
/// It automatically detects which type based on account properties.
///
/// # Arguments
///
/// * `program_id` - Our program's public key (for ownership checks)
/// * `expected_authority` - The pubkey we expect (from mint/account state)
/// * `authority_info` - The account provided as authority
/// * `signer_accounts` - Additional signer accounts (for multisig)
///
/// # Returns
///
/// * `Ok(())` - Authority is valid and has signed/authorized
/// * `Err(InvalidAuthority)` - Authority doesn't match expected
/// * `Err(MissingRequiredSignature)` - Single signer didn't sign
/// * `Err(NotEnoughSigners)` - Multisig lacks required signatures
///
/// # Single Signer Flow
///
/// ```text
/// 1. authority_info.key == expected_authority?
/// 2. authority_info.is_signer == true?
/// 3. If both yes → OK!
/// ```
///
/// # Multisig Flow
///
/// ```text
/// 1. authority_info.data_len() == 355?
/// 2. authority_info.owner == program_id?
/// 3. If yes to both, treat as multisig:
///    a. authority_info.key == expected_authority?
///    b. Load Multisig state
///    c. Count valid signers from signer_accounts
///    d. count >= multisig.m?
/// 4. If all checks pass → OK!
/// ```
///
/// # Example
///
/// ```ignore
/// // For MintTo instruction
/// let mint_authority = mint.mint_authority
///     .as_ref()
///     .ok_or(TokenError::MintAuthorityRequired)?;
///
/// validate_authority(
///     program_id,
///     mint_authority,     // Expected authority from mint state
///     authority_info,     // Provided by transaction
///     &signer_accounts,   // Additional signers for multisig
/// )?;
/// ```
pub fn validate_authority(
    program_id: &Pubkey,
    expected_authority: &Pubkey,
    authority_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
) -> ProgramResult {
    // =========================================================================
    // DETECT AUTHORITY TYPE
    // =========================================================================
    
    // Check if this might be a multisig account:
    // - Has exactly 355 bytes (Multisig::LEN)
    // - Is owned by our program
    let is_multisig = authority_info.data_len() == Multisig::LEN 
        && authority_info.owner == program_id;

    if is_multisig {
        // =====================================================================
        // MULTISIG VALIDATION PATH
        // =====================================================================
        validate_multisig(
            program_id,
            expected_authority,
            authority_info,
            signer_accounts,
        )
    } else {
        // =====================================================================
        // SINGLE SIGNER VALIDATION PATH
        // =====================================================================
        validate_single_signer(expected_authority, authority_info)
    }
}

// =============================================================================
// SINGLE SIGNER VALIDATION
// =============================================================================

/// Validate a single-signer authority.
///
/// Simple checks:
/// 1. Provided key matches expected
/// 2. Provided key has signed the transaction
///
/// # Arguments
///
/// * `expected_authority` - The pubkey we expect
/// * `authority_info` - The account claiming to be authority
///
/// # Returns
///
/// * `Ok(())` - Valid authority that has signed
/// * `Err(InvalidAuthority)` - Wrong key provided
/// * `Err(MissingRequiredSignature)` - Right key but didn't sign
fn validate_single_signer(
    expected_authority: &Pubkey,
    authority_info: &AccountInfo,
) -> ProgramResult {
    // =========================================================================
    // CHECK 1: Key matches expected
    // =========================================================================
    if authority_info.key != expected_authority {
        return Err(TokenError::InvalidAuthority.into());
    }

    // =========================================================================
    // CHECK 2: Has signed the transaction
    // =========================================================================
    if !authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok(())
}

// =============================================================================
// MULTISIG VALIDATION
// =============================================================================

/// Validate a multisig authority.
///
/// # Process
///
/// 1. Verify multisig account key matches expected authority
/// 2. Verify multisig account is owned by our program
/// 3. Load and verify multisig is initialized
/// 4. Count valid signers from signer_accounts
/// 5. Verify count >= m (required signatures)
///
/// # What Makes a Valid Signer?
///
/// A signer is counted if:
/// - The account has `is_signer = true` (actually signed)
/// - The account's pubkey is in `multisig.signers[0..n]`
///
/// # Arguments
///
/// * `program_id` - Our program's ID
/// * `expected_authority` - The expected multisig account pubkey
/// * `multisig_info` - The multisig account
/// * `signer_accounts` - Accounts that may have signed
///
/// # Returns
///
/// * `Ok(())` - Valid multisig with sufficient signatures
/// * `Err(InvalidAuthority)` - Multisig key doesn't match
/// * `Err(InvalidAccountOwner)` - Multisig not owned by us
/// * `Err(UninitializedAccount)` - Multisig not initialized
/// * `Err(NotEnoughSigners)` - Fewer than M valid signatures
fn validate_multisig(
    program_id: &Pubkey,
    expected_authority: &Pubkey,
    multisig_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
) -> ProgramResult {
    // =========================================================================
    // CHECK 1: Multisig account key matches expected
    // =========================================================================
    if multisig_info.key != expected_authority {
        return Err(TokenError::InvalidAuthority.into());
    }

    // =========================================================================
    // CHECK 2: Multisig is owned by our program
    // =========================================================================
    // This prevents fake multisig accounts from other programs
    if multisig_info.owner != program_id {
        return Err(TokenError::InvalidAccountOwner.into());
    }

    // =========================================================================
    // CHECK 3: Load and verify multisig state
    // =========================================================================
    let multisig = Multisig::unpack_from_slice(&multisig_info.data.borrow())?;

    if !multisig.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    // =========================================================================
    // CHECK 4: Count valid signers
    // =========================================================================
    let mut valid_signer_count: u8 = 0;

    for signer_account in signer_accounts {
        // Skip accounts that didn't actually sign
        if !signer_account.is_signer {
            continue;
        }

        // Check if this signer is in the multisig's signer list
        // Only check the first `n` signers (the valid ones)
        let is_valid_signer = multisig
            .signers
            .iter()
            .take(multisig.n as usize)
            .any(|stored_signer| stored_signer == signer_account.key);

        if is_valid_signer {
            // Increment counter with overflow protection
            valid_signer_count = valid_signer_count
                .checked_add(1)
                .ok_or(TokenError::Overflow)?;
        }
    }

    // =========================================================================
    // CHECK 5: Verify we have enough valid signers
    // =========================================================================
    if valid_signer_count < multisig.m {
        return Err(TokenError::NotEnoughSigners.into());
    }

    Ok(())
}

// =============================================================================
// OWNER OR DELEGATE VALIDATION
// =============================================================================

/// Validate owner or delegate authority for token account operations.
///
/// Many operations (Transfer, Burn) can be authorized by either:
/// - The token account's owner
/// - An approved delegate
///
/// This function tries owner first, then delegate.
///
/// # Returns
///
/// * `Ok(false)` - Owner authority was used
/// * `Ok(true)` - Delegate authority was used
/// * `Err(InvalidAuthority)` - Neither owner nor delegate
///
/// # Why Return a bool?
///
/// When delegate is used, the caller needs to:
/// 1. Check `delegated_amount >= amount`
/// 2. Decrement `delegated_amount`
/// 3. Clear delegate if `delegated_amount == 0`
///
/// The bool tells the caller which path was taken.
///
/// # Arguments
///
/// * `program_id` - Our program's ID
/// * `account_owner` - The token account's owner field
/// * `account_delegate` - The token account's delegate field (may be None)
/// * `authority_info` - The account claiming authority
/// * `signer_accounts` - Additional signers for multisig
///
/// # Example
///
/// ```ignore
/// let used_delegate = validate_owner_or_delegate(
///     program_id,
///     &source.owner,
///     source.delegate.as_ref(),
///     authority_info,
///     &signer_accounts,
/// )?;
///
/// if used_delegate {
///     // Check and decrement delegated_amount
///     if source.delegated_amount < amount {
///         return Err(TokenError::InsufficientDelegatedAmount.into());
///     }
///     source.delegated_amount -= amount;
///     if source.delegated_amount == 0 {
///         source.delegate = COption::none();
///     }
/// }
/// ```
pub fn validate_owner_or_delegate(
    program_id: &Pubkey,
    account_owner: &Pubkey,
    account_delegate: Option<&Pubkey>,
    authority_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
) -> Result<bool, ProgramError> {
    // =========================================================================
    // TRY 1: Validate as owner
    // =========================================================================
    if validate_authority(
        program_id,
        account_owner,
        authority_info,
        signer_accounts,
    )
    .is_ok()
    {
        return Ok(false); // false = owner was used
    }

    // =========================================================================
    // TRY 2: Validate as delegate (if present)
    // =========================================================================
    if let Some(delegate) = account_delegate {
        if validate_authority(
            program_id,
            delegate,
            authority_info,
            signer_accounts,
        )
        .is_ok()
        {
            return Ok(true); // true = delegate was used
        }
    }

    // =========================================================================
    // NEITHER WORKED
    // =========================================================================
    Err(TokenError::InvalidAuthority.into())
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a mock AccountInfo for testing
    fn create_test_account_info<'a>(
        key: &'a Pubkey,
        is_signer: bool,
        is_writable: bool,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        owner: &'a Pubkey,
    ) -> AccountInfo<'a> {
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            false, // executable
            0,     // rent_epoch
        )
    }

    // =========================================================================
    // SINGLE SIGNER TESTS
    // =========================================================================

    #[test]
    fn test_single_signer_valid() {
        let key = Pubkey::new_unique();
        let mut lamports = 0u64;
        let mut data = vec![];
        let owner = Pubkey::new_unique();

        let account = create_test_account_info(
            &key,
            true, // is_signer = true
            false,
            &mut lamports,
            &mut data,
            &owner,
        );

        // Should succeed: key matches and is signer
        let result = validate_single_signer(&key, &account);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_signer_wrong_key() {
        let expected_key = Pubkey::new_unique();
        let wrong_key = Pubkey::new_unique();
        let mut lamports = 0u64;
        let mut data = vec![];
        let owner = Pubkey::new_unique();

        let account = create_test_account_info(
            &wrong_key, // Different key!
            true,
            false,
            &mut lamports,
            &mut data,
            &owner,
        );

        // Should fail: key doesn't match
        let result = validate_single_signer(&expected_key, &account);
        assert!(result.is_err());
    }

    #[test]
    fn test_single_signer_not_signer() {
        let key = Pubkey::new_unique();
        let mut lamports = 0u64;
        let mut data = vec![];
        let owner = Pubkey::new_unique();

        let account = create_test_account_info(
            &key,
            false, // is_signer = false
            false,
            &mut lamports,
            &mut data,
            &owner,
        );

        // Should fail: didn't sign
        let result = validate_single_signer(&key, &account);
        assert!(result.is_err());
    }

    // =========================================================================
    // OWNER OR DELEGATE TESTS
    // =========================================================================

    #[test]
    fn test_owner_or_delegate_owner_valid() {
        let program_id = Pubkey::new_unique();
        let owner_key = Pubkey::new_unique();
        let mut lamports = 0u64;
        let mut data = vec![];
        let random_owner = Pubkey::new_unique();

        let authority = create_test_account_info(
            &owner_key,
            true,
            false,
            &mut lamports,
            &mut data,
            &random_owner,
        );

        let result = validate_owner_or_delegate(
            &program_id,
            &owner_key,
            None, // No delegate
            &authority,
            &[],
        );

        // Should succeed and return false (owner used)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_owner_or_delegate_delegate_valid() {
        let program_id = Pubkey::new_unique();
        let owner_key = Pubkey::new_unique();
        let delegate_key = Pubkey::new_unique();
        let mut lamports = 0u64;
        let mut data = vec![];
        let random_owner = Pubkey::new_unique();

        // Authority is the delegate, not the owner
        let authority = create_test_account_info(
            &delegate_key,
            true,
            false,
            &mut lamports,
            &mut data,
            &random_owner,
        );

        let result = validate_owner_or_delegate(
            &program_id,
            &owner_key,
            Some(&delegate_key),
            &authority,
            &[],
        );

        // Should succeed and return true (delegate used)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_owner_or_delegate_neither() {
        let program_id = Pubkey::new_unique();
        let owner_key = Pubkey::new_unique();
        let delegate_key = Pubkey::new_unique();
        let random_key = Pubkey::new_unique();
        let mut lamports = 0u64;
        let mut data = vec![];
        let random_owner = Pubkey::new_unique();

        // Authority is neither owner nor delegate
        let authority = create_test_account_info(
            &random_key,
            true,
            false,
            &mut lamports,
            &mut data,
            &random_owner,
        );

        let result = validate_owner_or_delegate(
            &program_id,
            &owner_key,
            Some(&delegate_key),
            &authority,
            &[],
        );

        // Should fail: neither owner nor delegate
        assert!(result.is_err());
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

AUTHORITY CONCEPT IN SPL TOKEN
==============================

Many operations require "authority":
- MintTo: needs mint_authority
- Transfer: needs owner (or delegate)
- Burn: needs owner (or delegate)
- FreezeAccount: needs freeze_authority
- CloseAccount: needs close_authority (or owner)

Authority can be:
1. A regular keypair (most common)
2. A multisig account (for shared control)
3. A PDA (for program-controlled)

SINGLE SIGNER FLOW
==================

User Transaction:
1. Create instruction with accounts
2. Sign with private key
3. Submit to network

Runtime:
1. Verify signature
2. Set is_signer = true for signed accounts
3. Call our program

Our validation:
1. Check authority_info.key == expected
2. Check authority_info.is_signer == true
3. If both true → authorized!

MULTISIG FLOW
=============

Structure:
- Multisig account stores N signer pubkeys
- Requires M signatures to authorize
- M ≤ N ≤ 11

Transaction:

Validation:
1. Detect multisig (size 355, owned by our program)
2. Load Multisig state from multisig_account
3. For each signer account:
   - Is it actually a signer?
   - Is it in multisig.signers[0..n]?
4. Count valid signers
5. Check count >= m

WHY DETECT BY SIZE?
===================

Q: How do we know if authority is multisig or single?

Option A: Add flag to instruction data
- Pros: Explicit
- Cons: Extra complexity, user error

Option B: Detect by account properties (our approach)
- Multisig accounts are ALWAYS 355 bytes
- Multisig accounts are ALWAYS owned by our program
- Regular signers don't match these criteria
- Automatic detection!

OWNER VS DELEGATE
=================

Token accounts have two potential authorities:

Owner (source.owner):
- Primary authority
- Can transfer unlimited tokens
- Can approve/revoke delegates
- Can close account

Delegate (source.delegate):
- Secondary, limited authority
- Can only transfer up to delegated_amount
- Set by owner via Approve
- Cleared when exhausted or via Revoke

For Transfer/Burn, either can authorize.
We return bool to indicate which:
- false = owner (no limit tracking needed)
- true = delegate (must decrement delegated_amount)

SECURITY CONSIDERATIONS
=======================

1. Always check ownership before trusting multisig
   - Fake multisig from attacker's program
   - Would have attacker-controlled signers
   - We verify owner == our program_id

2. Only count actual signers
   - Attacker could include non-signers
   - We check is_signer for each
   - Only signed accounts count

3. Only check first N signers
   - multisig.signers has 11 slots
   - Only first N are valid
   - Remaining are garbage/zeros

4. Overflow protection when counting
   - Technically can't overflow with max 11 signers
   - But we check anyway (defense in depth)

EXAMPLE: 2-OF-3 MULTISIG MINT
=============================

Setup:

MintTo Transaction:

Validation:
1. mint.mint_authority == multisig_account.key ✓
2. multisig_account.data_len() == 355 ✓
3. multisig_account.owner == program_id ✓
4. Load Multisig state
5. alice_account.is_signer? Yes
6. alice_account.key in [Alice, Bob, Carol]? Yes (match 0)
7. bob_account.is_signer? Yes
8. bob_account.key in [Alice, Bob, Carol]? Yes (match 1)
9. valid_count = 2
10. 2 >= m(2) ✓
11. Authorized!
*/