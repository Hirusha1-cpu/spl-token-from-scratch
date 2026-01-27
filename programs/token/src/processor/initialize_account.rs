//! InitializeAccount Instruction Processor
//!
//! Creates a new token account (wallet for a specific token).

use crate::error::TokenError;
use crate::state::{Account, AccountState, COption, Mint, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Process InitializeAccount instruction
///
/// Accounts expected:
/// 0. `[writable]` Token account to initialize
/// 1. `[]` Mint this account will hold
/// 2. `[]` Owner of the new account
/// 3. `[]` Rent sysvar
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Token account
    let account_info = next_account_info(account_info_iter)?;

    // Account 1: Mint
    let mint_info = next_account_info(account_info_iter)?;

    // Account 2: Owner
    let owner_info = next_account_info(account_info_iter)?;

    // Account 3: Rent sysvar
    let rent_info = next_account_info(account_info_iter)?;
    let rent = Rent::from_account_info(rent_info)?;

    // Validate token account
    assert_owned_by(account_info, program_id)?;
    assert_writable(account_info)?;
    assert_data_length(account_info, Account::LEN)?;
    assert_rent_exempt(&rent, account_info)?;

    // Validate mint
    assert_owned_by(mint_info, program_id)?;
    assert_data_length(mint_info, Mint::LEN)?;

    // Load and verify mint is initialized
    let mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;
    if !mint.is_initialized {
        return Err(TokenError::UninitializedAccount.into());
    }

    // Load token account
    let mut account = Account::unpack_from_slice(&account_info.data.borrow())?;

    // Prevent double initialization
    if account.is_initialized() {
        return Err(TokenError::AlreadyInitialized.into());
    }

    // Initialize account
    account.mint = *mint_info.key;
    account.owner = *owner_info.key;
    account.amount = 0;
    account.delegate = COption::none();
    account.state = AccountState::Initialized;
    account.is_native = COption::none();
    account.delegated_amount = 0;
    account.close_authority = COption::none();

    // Save account
    account.pack_into_slice(&mut account_info.data.borrow_mut())?;

    Ok(())
}