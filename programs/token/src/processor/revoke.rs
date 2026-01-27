//! Revoke Instruction Processor
//!
//! Revokes a delegate's approval.

use crate::error::TokenError;
use crate::state::{Account, COption, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process Revoke instruction
///
/// Accounts expected:
/// 0. `[writable]` Source token account
/// 1. `[signer]` Owner
/// 2..2+M. `[signer]` Multisig signers (if applicable)
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Source token account
    let source_info = next_account_info(account_info_iter)?;

    // Account 1: Owner
    let owner_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate source account
    assert_owned_by(source_info, program_id)?;
    assert_writable(source_info)?;
    assert_data_length(source_info, Account::LEN)?;

    // Load source account
    let mut source = Account::unpack_from_slice(&source_info.data.borrow())?;

    // Validate initialization
    if !source.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Validate owner authority
    validate_authority(
        program_id,
        &source.owner,
        owner_info,
        &signer_accounts,
    )?;

    // Revoke delegate
    source.delegate = COption::none();
    source.delegated_amount = 0;

    // Save source
    source.pack_into_slice(&mut source_info.data.borrow_mut())?;

    Ok(())
}