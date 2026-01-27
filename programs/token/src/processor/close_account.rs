//! CloseAccount Instruction Processor
//!
//! Closes a token account and reclaims the rent.

use crate::error::TokenError;
use crate::state::{Account, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process CloseAccount instruction
///
/// Accounts expected:
/// 0. `[writable]` Token account to close
/// 1. `[writable]` Destination for rent lamports
/// 2. `[signer]` Close authority or owner
/// 3..3+M. `[signer]` Multisig signers (if applicable)
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Token account to close
    let account_info = next_account_info(account_info_iter)?;

    // Account 1: Destination for lamports
    let dest_info = next_account_info(account_info_iter)?;

    // Account 2: Authority
    let authority_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate account to close
    assert_owned_by(account_info, program_id)?;
    assert_writable(account_info)?;
    assert_data_length(account_info, Account::LEN)?;

    // Validate destination
    assert_writable(dest_info)?;

    // Cannot close into self
    if account_info.key == dest_info.key {
        return Err(TokenError::InvalidAuthority.into());
    }

    // Load account
    let account = Account::unpack_from_slice(&account_info.data.borrow())?;

    // Validate initialization
    if !account.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Must have zero balance
    if account.amount != 0 {
        return Err(TokenError::NonZeroBalance.into());
    }

    // Validate authority (close_authority or owner)
    let close_authority = account
        .close_authority
        .as_ref()
        .unwrap_or(&account.owner);

    validate_authority(
        program_id,
        close_authority,
        authority_info,
        &signer_accounts,
    )?;

    // Transfer lamports to destination
    let account_lamports = account_info.lamports();
    **dest_info.lamports.borrow_mut() = dest_info
        .lamports()
        .checked_add(account_lamports)
        .ok_or(TokenError::Overflow)?;
    **account_info.lamports.borrow_mut() = 0;

    // Zero out account data
    let mut account_data = account_info.data.borrow_mut();
    account_data.fill(0);

    Ok(())
}