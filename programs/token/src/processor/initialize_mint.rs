//! InitializeMint Instruction Processor
//!
//! Creates a new token mint (defines a new token type).

use crate::error::TokenError;
use crate::state::{COption, Mint, Pack};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Process InitializeMint instruction
///
/// Accounts expected:
/// 0. `[writable]` Mint account to initialize
/// 1. `[]` Rent sysvar
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Mint account
    let mint_info = next_account_info(account_info_iter)?;

    // Account 1: Rent sysvar
    let rent_info = next_account_info(account_info_iter)?;
    let rent = Rent::from_account_info(rent_info)?;

    // Validate mint account
    assert_owned_by(mint_info, program_id)?;
    assert_writable(mint_info)?;
    assert_data_length(mint_info, Mint::LEN)?;
    assert_rent_exempt(&rent, mint_info)?;

    // Load mint
    let mut mint = Mint::unpack_from_slice(&mint_info.data.borrow())?;

    // Prevent double initialization
    if mint.is_initialized {
        return Err(TokenError::AlreadyInitialized.into());
    }

    // Initialize mint
    mint.mint_authority = COption::some(mint_authority);
    mint.supply = 0;
    mint.decimals = decimals;
    mint.is_initialized = true;
    mint.freeze_authority = freeze_authority.into();

    // Save mint
    mint.pack_into_slice(&mut mint_info.data.borrow_mut())?;

    Ok(())
}