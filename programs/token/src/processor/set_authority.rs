//! SetAuthority Instruction Processor
//!
//! Changes an authority on a mint or token account.

use crate::error::TokenError;
use crate::instruction::AuthorityType;
use crate::state::{Account, COption, Mint, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Process SetAuthority instruction
///
/// Accounts expected:
/// 0. `[writable]` Mint or token account
/// 1. `[signer]` Current authority
/// 2..2+M. `[signer]` Multisig signers (if applicable)
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authority_type: AuthorityType,
    new_authority: Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Mint or token account
    let account_info = next_account_info(account_info_iter)?;

    // Account 1: Current authority
    let authority_info = next_account_info(account_info_iter)?;

    // Remaining: Multisig signers
    let signer_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Validate account
    assert_owned_by(account_info, program_id)?;
    assert_writable(account_info)?;

    // Route based on authority type
    match authority_type {
        AuthorityType::MintTokens => {
            process_set_mint_authority(
                program_id,
                account_info,
                authority_info,
                &signer_accounts,
                new_authority,
            )
        }
        AuthorityType::FreezeAccount => {
            process_set_freeze_authority(
                program_id,
                account_info,
                authority_info,
                &signer_accounts,
                new_authority,
            )
        }
        AuthorityType::AccountOwner => {
            process_set_account_owner(
                program_id,
                account_info,
                authority_info,
                &signer_accounts,
                new_authority,
            )
        }
        AuthorityType::CloseAccount => {
            process_set_close_authority(
                program_id,
                account_info,
                authority_info,
                &signer_accounts,
                new_authority,
            )
        }
    }
}

fn process_set_mint_authority(
    program_id: &Pubkey,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
    new_authority: Option<Pubkey>,
) -> ProgramResult {
    assert_data_length(mint_info, Mint::LEN)?;

    let mut mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;

    if !mint.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    let current_authority = mint
        .mint_authority
        .as_ref()
        .ok_or(TokenError::InvalidAuthority)?;

    validate_authority(program_id, current_authority, authority_info, signer_accounts)?;

    mint.mint_authority = new_authority.into();
    mint.pack_into_slice(&mut mint_info.data.borrow_mut())?;

    Ok(())
}

fn process_set_freeze_authority(
    program_id: &Pubkey,
    mint_info: &AccountInfo,
    authority_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
    new_authority: Option<Pubkey>,
) -> ProgramResult {
    assert_data_length(mint_info, Mint::LEN)?;

    let mut mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;

    if !mint.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    let current_authority = mint
        .freeze_authority
        .as_ref()
        .ok_or(TokenError::FreezeAuthorityRequired)?;

    validate_authority(program_id, current_authority, authority_info, signer_accounts)?;

    mint.freeze_authority = new_authority.into();
    mint.pack_into_slice(&mut mint_info.data.borrow_mut())?;

    Ok(())
}

fn process_set_account_owner(
    program_id: &Pubkey,
    account_info: &AccountInfo,
    authority_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
    new_authority: Option<Pubkey>,
) -> ProgramResult {
    assert_data_length(account_info, Account::LEN)?;

    let mut account = Account::unpack_from_slice(&account_info.data.borrow())?;

    if !account.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }

    validate_authority(program_id, &account.owner, authority_info, signer_accounts)?;

    // Owner cannot be set to None
    let new_owner = new_authority.ok_or(TokenError::InvalidAuthority)?;

    account.owner = new_owner;

    // Clear delegate when owner changes
    account.delegate = COption::none();
    account.delegated_amount = 0;

    account.pack_into_slice(&mut account_info.data.borrow_mut())?;

    Ok(())
}

fn process_set_close_authority(
    program_id: &Pubkey,
    account_info: &AccountInfo,
    authority_info: &AccountInfo,
    signer_accounts: &[AccountInfo],
    new_authority: Option<Pubkey>,
) -> ProgramResult {
    assert_data_length(account_info, Account::LEN)?;

    let mut account = Account::unpack_from_slice(&account_info.data.borrow())?;

    if !account.is_initialized() {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Current close authority defaults to owner
    let current_authority = account
        .close_authority
        .as_ref()
        .unwrap_or(&account.owner);

    validate_authority(program_id, current_authority, authority_info, signer_accounts)?;

    account.close_authority = new_authority.into();
    account.pack_into_slice(&mut account_info.data.borrow_mut())?;

    Ok(())
}