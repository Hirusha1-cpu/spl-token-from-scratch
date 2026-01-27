//! FreezeAccount Instruction Processor
//!
//! Freezes a token account, preventing transfers out.

use crate::error::TokenError;
use crate::state::{Account, AccountState, Mint, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process FreezeAccount instruction
///
/// Accounts expected:
/// 0. `[writable]` Token account to freeze
/// 1. `[]` Mint
/// 2. `[signer]` Freeze authority
/// 3..3+M. `[signer]` Multisig signers (if applicable)
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Token account to freeze
    let account_info = next_account_info(account_info_iter)?;

    // Account 1: Mint
    let mint_info = next_account_info(account_info_iter)?;

    // Account 2: Freeze authority
    let authority_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate token account
    assert_owned_by(account_info, program_id)?;
    assert_writable(account_info)?;
    assert_data_length(account_info, Account::LEN)?;

    // Validate mint
    assert_owned_by(mint_info, program_id)?;
    assert_data_length(mint_info, Mint::LEN)?;

    // Load states
    let mut account = Account::unpack_from_slice(&account_info.data.borrow())?;
    let mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;

    // Validate initialization
    if !account.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }
    if !mint.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Validate account is for this mint
    if account.mint != *mint_info.key {
        return Err(TokenError::MintMismatch.into());
    }

    // Get freeze authority
    let freeze_authority = mint
        .freeze_authority
        .as_ref()
        .ok_or(TokenError::FreezeAuthorityRequired)?;

    // Validate authority
    validate_authority(
        program_id,
        freeze_authority,
        authority_info,
        &signer_accounts,
    )?;

    // Freeze the account
    account.state = AccountState::Frozen;

    // Save account
    account.pack_into_slice(&mut account_info.data.borrow_mut())?;

    Ok(())
}