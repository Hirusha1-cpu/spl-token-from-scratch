//! Program Entrypoint
//!
//! This module defines the entrypoint for the Solana program.
//! The entrypoint is where the Solana runtime calls into our program
//! when a transaction includes an instruction for us.
//!
//! Think of it like the `main()` function, but for on-chain programs.

// =============================================================================
// CONDITIONAL COMPILATION
// =============================================================================

// Only compile this module if the "no-entrypoint" feature is NOT enabled
// This allows other programs to use our crate without entrypoint conflicts
#![cfg(not(feature = "no-entrypoint"))]

// =============================================================================
// IMPORTS
// =============================================================================

use crate::processor::Processor;
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

// =============================================================================
// ENTRYPOINT DECLARATION
// =============================================================================

// This macro generates the actual entrypoint that Solana looks for
// It handles:
// - Setting up the heap allocator
// - Deserializing accounts from raw memory
// - Calling our function with proper types
// - Converting our Result to what Solana expects
entrypoint!(process_instruction);

// =============================================================================
// ENTRYPOINT FUNCTION
// =============================================================================

/// The main entrypoint for the token program.
///
/// This function is called by the Solana runtime for every instruction
/// sent to our program.
///
/// # Arguments
///
/// * `program_id` - The public key of this program (our deployed address)
/// * `accounts` - Slice of all accounts involved in this instruction
/// * `instruction_data` - The raw bytes of instruction-specific data
///
/// # Returns
///
/// * `Ok(())` - Instruction executed successfully
/// * `Err(ProgramError)` - Something went wrong
///
/// # Account Ownership
///
/// The runtime only passes accounts that are either:
/// - Owned by this program
/// - Signers of the transaction
/// - Being read (not modified)
///
/// # Example Flow
///
/// 1. User creates a transaction with instruction data
/// 2. Transaction is sent to Solana network
/// 3. Validator deserializes transaction
/// 4. For each instruction, validator calls the program's entrypoint
/// 5. We receive: program_id, accounts, instruction_data
/// 6. We process and return success or error
/// 7. If error, entire transaction is rolled back
///
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Delegate to our processor
    // This separation makes the code more organized and testable
    Processor::process(program_id, accounts, instruction_data)
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

THE ENTRYPOINT MACRO
====================

entrypoint!(process_instruction);

This single line does A LOT:

1. Creates the actual `entrypoint` symbol that Solana looks for
2. Sets up a custom allocator for the BPF heap
3. Sets up a custom panic handler
4. Deserializes the raw input buffer into Rust types

What Solana actually passes (raw):
- A pointer to a buffer containing serialized data
- Everything is packed tightly in memory

What the macro gives us:
- program_id: &Pubkey (32 bytes, our address)
- accounts: &[AccountInfo] (variable length array)
- instruction_data: &[u8] (variable length bytes)

ACCOUNTINFO STRUCTURE
=====================

Each AccountInfo contains:

pub struct AccountInfo<'a> {
    pub key: &'a Pubkey,              // Account's address
    pub is_signer: bool,              // Did this account sign?
    pub is_writable: bool,            // Can we modify this account?
    pub lamports: Rc<RefCell<&'a mut u64>>,  // SOL balance
    pub data: Rc<RefCell<&'a mut [u8]>>,     // Account data
    pub owner: &'a Pubkey,            // Program that owns this account
    pub executable: bool,             // Is this a program?
    pub rent_epoch: u64,              // Rent epoch
}

Key points:
- lamports and data use RefCell for interior mutability
- We can modify them even with shared references
- The runtime checks our modifications after execution

PROGRAM RESULT
==============

pub type ProgramResult = Result<(), ProgramError>;

On success: return Ok(())
On failure: return Err(ProgramError::Something)

If we return an error:
- Transaction fails
- All state changes are reverted
- User's transaction fee is still consumed (spam prevention)

WHY SEPARATE PROCESSOR?
=======================

We could put all logic directly in process_instruction.
But separating it:
1. Makes testing easier (can test Processor without entrypoint)
2. Keeps entrypoint minimal
3. Follows single responsibility principle
4. Matches SPL Token's structure

CONDITIONAL COMPILATION
=======================

#![cfg(not(feature = "no-entrypoint"))]

This entire file is only compiled when the feature is NOT set.

When would you set it?
- Another program depends on your crate
- They want to use your types/instructions
- But they have their own entrypoint
- Two entrypoints = compile error

Example Cargo.toml in another program:
[dependencies]
spl_token_from_scratch = { path = "../", features = ["no-entrypoint"] }

THE FLOW VISUALIZED
===================

User Transaction
     │
     ▼
┌─────────────────┐
│ Solana Runtime  │
└────────┬────────┘
         │ Deserialize & Call
         ▼
┌─────────────────────────────┐
│ entrypoint!(process_instruction) │
│   - Sets up allocator           │
│   - Parses raw buffer           │
│   - Calls our function          │
└────────────┬────────────────────┘
             │
             ▼
┌─────────────────────────────┐
│ process_instruction()       │
│   - Receives typed data     │
│   - Delegates to Processor  │
└────────────┬────────────────┘
             │
             ▼
┌─────────────────────────────┐
│ Processor::process()        │
│   - Parses instruction      │
│   - Routes to handler       │
│   - Executes business logic │
└─────────────────────────────┘
*/