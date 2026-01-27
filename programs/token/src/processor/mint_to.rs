//! MintTo Instruction Processor
//!
//! Mints new tokens to a token account.

use crate::error::TokenError;
use crate::state::{Account, Mint, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process MintTo instruction
///
/// Accounts expected:
/// 0. `[writable]` Mint
/// 1. `[writable]` Destination token account
/// 2. `[signer]` Mint authority
/// 3..3+M. `[signer]` Multisig signers (if applicable)
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Mint
    let mint_info = next_account_info(account_info_iter)?;

    // Account 1: Destination
    let dest_info = next_account_info(account_info_iter)?;

    // Account 2: Authority
    let authority_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate mint
    assert_owned_by(mint_info, program_id)?;
    assert_writable(mint_info)?;
    assert_data_length(mint_info, Mint::LEN)?;

    // Validate destination
    assert_owned_by(dest_info, program_id)?;
    assert_writable(dest_info)?;
    assert_data_length(dest_info, Account::LEN)?;

    // Load states
    let mut mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;
    let mut dest_account = Account::unpack_from_slice(&dest_info.data.borrow())?;

    // Validate mint is initialized
    if !mint.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Validate destination is initialized
    if !dest_account.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Validate destination is not frozen
    if dest_account.is_frozen() {
        return Err(TokenError::AccountFrozen.into());
    }

    // Validate destination mint matches
    if dest_account.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    // Get and validate mint authority
    let mint_authority = mint
        .mint_authority
        .as_ref()
        .ok_or(TokenError::MintAuthorityRequired)?;

    validate_authority(
        program_id,
        mint_authority,
        authority_info,
        &signer_accounts,
    )?;

    // Update balances
    mint.supply = checked_add(mint.supply, amount)?;
    dest_account.amount = checked_add(dest_account.amount, amount)?;

    // Save states
    mint.pack_into_slice(&mut mint_info.data.borrow_mut())?;
    dest_account.pack_into_slice(&mut dest_info.data.borrow_mut())?;

    Ok(())
}