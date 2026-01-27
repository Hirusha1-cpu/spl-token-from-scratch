//! Instruction Processors
//!
//! This module contains the business logic for each instruction.
//! Each instruction has its own file for clarity and maintainability.

pub mod approve;
pub mod burn;
pub mod close_account;
pub mod freeze_account;
pub mod initialize_account;
pub mod initialize_mint;
pub mod initialize_multisig;
pub mod mint_to;
pub mod revoke;
pub mod set_authority;
pub mod thaw_account;
pub mod transfer;

use crate::instruction::TokenInstruction;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

/// Main processor that routes instructions to specific handlers
pub struct Processor;

impl Processor {
    /// Process a Token program instruction
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        // Parse the instruction
        let instruction = TokenInstruction::unpack(instruction_data)?;

        // Route to appropriate handler
        match instruction {
            TokenInstruction::InitializeMint {
                decimals,
                mint_authority,
                freeze_authority,
            } => {
                msg!("Instruction: InitializeMint");
                initialize_mint::process(
                    program_id,
                    accounts,
                    decimals,
                    mint_authority,
                    freeze_authority,
                )
            }

            TokenInstruction::InitializeAccount => {
                msg!("Instruction: InitializeAccount");
                initialize_account::process(program_id, accounts)
            }

            TokenInstruction::InitializeMultisig { m } => {
                msg!("Instruction: InitializeMultisig");
                initialize_multisig::process(program_id, accounts, m)
            }

            TokenInstruction::Transfer { amount } => {
                msg!("Instruction: Transfer");
                transfer::process(program_id, accounts, amount)
            }

            TokenInstruction::Approve { amount } => {
                msg!("Instruction: Approve");
                approve::process(program_id, accounts, amount)
            }

            TokenInstruction::Revoke => {
                msg!("Instruction: Revoke");
                revoke::process(program_id, accounts)
            }

            TokenInstruction::SetAuthority {
                authority_type,
                new_authority,
            } => {
                msg!("Instruction: SetAuthority");
                set_authority::process(program_id, accounts, authority_type, new_authority)
            }

            TokenInstruction::MintTo { amount } => {
                msg!("Instruction: MintTo");
                mint_to::process(program_id, accounts, amount)
            }

            TokenInstruction::Burn { amount } => {
                msg!("Instruction: Burn");
                burn::process(program_id, accounts, amount)
            }

            TokenInstruction::CloseAccount => {
                msg!("Instruction: CloseAccount");
                close_account::process(program_id, accounts)
            }

            TokenInstruction::FreezeAccount => {
                msg!("Instruction: FreezeAccount");
                freeze_account::process(program_id, accounts)
            }

            TokenInstruction::ThawAccount => {
                msg!("Instruction: ThawAccount");
                thaw_account::process(program_id, accounts)
            }
        }
    }
}