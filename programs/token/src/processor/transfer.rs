//! Transfer Instruction Processor
//!
//! Transfers tokens from one account to another.

use crate::error::TokenError;
use crate::state::{Account, COption, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process Transfer instruction
///
/// Accounts expected:
/// 0. `[writable]` Source token account
/// 1. `[writable]` Destination token account
/// 2. `[signer]` Owner or delegate
/// 3..3+M. `[signer]` Multisig signers (if applicable)
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Source
    let source_info = next_account_info(account_info_iter)?;

    // Account 1: Destination
    let dest_info = next_account_info(account_info_iter)?;

    // Account 2: Authority
    let authority_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate source
    assert_owned_by(source_info, program_id)?;
    assert_writable(source_info)?;
    assert_data_length(source_info, Account::LEN)?;

    // Validate destination
    assert_owned_by(dest_info, program_id)?;
    assert_writable(dest_info)?;
    assert_data_length(dest_info, Account::LEN)?;

    // Prevent self-transfer
    if source_info.key == dest_info.key {
        return Err(TokenError::SelfTransfer.into());
    }

    // Load states
    let mut source = Account::unpack_from_slice(&source_info.data.borrow())?;
    let mut dest = Account::unpack_from_slice(&dest_info.data.borrow())?;

    // Validate initialization
    if !source.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }
    if !dest.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Validate not frozen
    if source.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }
    if dest.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    // Validate mints match
    if source.mint != dest.mint {
        return Err(TokenError::MintMismatch.into());
    }

    // Validate sufficient funds
    if source.amount < amount {
        return Err(TokenError::InsufficientFunds.into());
    }

    // Validate authority
    let delegate_pubkey: Option<&Pubkey> = source.delegate.as_ref();
    let used_delegate = validate_owner_or_delegate(
        program_id,
        &source.owner,
        delegate_pubkey,
        authority_info,
        &signer_accounts,
    )?;

    // Handle delegate allowance
    if used_delegate {
        if source.delegated_amount < amount {
            return Err(TokenError::InsufficientDelegatedAmount.into());
        }
        source.delegated_amount = checked_sub(source.delegated_amount, amount)?;
        if source.delegated_amount == 0 {
            source.delegate = COption::none();
        }
    }

    // Transfer tokens
    source.amount = checked_sub(source.amount, amount)?;
    dest.amount = checked_add(dest.amount, amount)?;

    // Save states
    source.pack_into_slice(&mut source_info.data.borrow_mut())?;
    dest.pack_into_slice(&mut dest_info.data.borrow_mut())?;

    Ok(())
}