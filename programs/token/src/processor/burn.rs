//! Burn Instruction Processor
//!
//! Burns (destroys) tokens, decreasing supply.

use crate::error::TokenError;
use crate::state::{Account, COption, Mint, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process Burn instruction
///
/// Accounts expected:
/// 0. `[writable]` Token account to burn from
/// 1. `[writable]` Mint
/// 2. `[signer]` Owner or delegate
/// 3..3+M. `[signer]` Multisig signers (if applicable)
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Token account
    let account_info = next_account_info(account_info_iter)?;

    // Account 1: Mint
    let mint_info = next_account_info(account_info_iter)?;

    // Account 2: Authority
    let authority_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate token account
    assert_owned_by(account_info, program_id)?;
    assert_writable(account_info)?;
    assert_data_length(account_info, Account::LEN)?;

    // Validate mint
    assert_owned_by(mint_info, program_id)?;
    assert_writable(mint_info)?;
    assert_data_length(mint_info, Mint::LEN)?;

    // Load states
    let mut account = Account::unpack_from_slice(&account_info.data.borrow())?;
    let mut mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;

    // Validate initialization
    if !account.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }
    if !mint.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Validate not frozen
    if account.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    // Validate mint matches
    if account.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    // Validate sufficient funds
    if account.amount < amount {
        return Err(TokenError::InsufficientFunds.into());
    }

    // Validate authority
    let delegate_pubkey: Option<&Pubkey> = account.delegate.as_ref();
    let used_delegate = validate_owner_or_delegate(
        program_id,
        &account.owner,
        delegate_pubkey,
        authority_info,
        &signer_accounts,
    )?;

    // Handle delegate allowance
    if used_delegate {
        if account.delegated_amount < amount {
            return Err(TokenError::InsufficientDelegatedAmount.into());
        }
        account.delegated_amount = checked_sub(account.delegated_amount, amount)?;
        if account.delegated_amount == 0 {
            account.delegate = COption::none();
        }
    }

    // Burn tokens
    account.amount = checked_sub(account.amount, amount)?;
    mint.supply = checked_sub(mint.supply, amount)?;

    // Save states
    account.pack_into_slice(&mut account_info.data.borrow_mut())?;
    mint.pack_into_slice(&mut mint_info.data.borrow_mut())?;

    Ok(())
}