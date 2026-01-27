//! InitializeMultisig Instruction Processor
//!
//! Creates a new multisig authority (M-of-N).

use crate::error::TokenError;
use crate::state::{Multisig, Pack, MAX_SIGNERS};
use crate::utils::*;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Process InitializeMultisig instruction
///
/// Accounts expected:
/// 0. `[writable]` Multisig account to initialize
/// 1. `[]` Rent sysvar
/// 2..2+N. `[]` Signer accounts
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], m: u8) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // Account 0: Multisig account
    let multisig_info = next_account_info(account_info_iter)?;

    // Account 1: Rent sysvar
    let rent_info = next_account_info(account_info_iter)?;
    let rent = Rent::from_account_info(rent_info)?;

    // Remaining accounts: Signers
    let signer_infos: Vec<&AccountInfo> = account_info_iter.collect();

    // Validate multisig account
    assert_owned_by(multisig_info, program_id)?;
    assert_writable(multisig_info)?;
    assert_data_length(multisig_info, Multisig::LEN)?;
    assert_rent_exempt(&rent, multisig_info)?;

    // Validate signer count
    let n = signer_infos.len();
    if n < 1 || n > MAX_SIGNERS {
        return Err(TokenError::InvalidMultisigConfig.into());
    }

    // Validate m
    if m < 1 || m as usize > n {
        return Err(TokenError::InvalidMultisigConfig.into());
    }

    // Load multisig
    let mut multisig = Multisig::unpack_from_slice(&multisig_info.data.borrow())?;

    // Prevent double initialization
    if multisig.is_initialized {
        return Err(TokenError::AlreadyInitialized.into());
    }

    // Initialize multisig
    multisig.m = m;
    multisig.n = n as u8;
    multisig.is_initialized = true;

    // Copy signer pubkeys
    for (i, signer_info) in signer_infos.iter().enumerate() {
        multisig.signers[i] = *signer_info.key;
    }

    // Clear remaining slots
    for i in n..MAX_SIGNERS {
        multisig.signers[i] = Pubkey::default();
    }

    // Save multisig
    multisig.pack_into_slice(&mut multisig_info.data.borrow_mut())?;

    Ok(())
}