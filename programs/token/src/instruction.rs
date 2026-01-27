//! Instruction Types
//!
//! This module defines all instructions supported by the token program.
//! Each instruction has:
//! - A discriminant (first byte, identifies the instruction type)
//! - Instruction-specific data (remaining bytes)
//! - Expected accounts (documented, not encoded in data)
//!
//! # Instruction Format
//!
//! ```text
//! [discriminant: u8][data: varies]
//! ```
//!
//! # Discriminant Values (matching SPL Token)
//!
//! | Value | Instruction |
//! |-------|-------------|
//! | 0 | InitializeMint |
//! | 1 | InitializeAccount |
//! | 2 | InitializeMultisig |
//! | 3 | Transfer |
//! | 4 | Approve |
//! | 5 | Revoke |
//! | 6 | SetAuthority |
//! | 7 | MintTo |
//! | 8 | Burn |
//! | 9 | CloseAccount |
//! | 10 | FreezeAccount |
//! | 11 | ThawAccount |

use crate::error::TokenError;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

// =============================================================================
// AUTHORITY TYPE
// =============================================================================

/// Types of authority that can be changed with SetAuthority instruction.
///
/// # Values
///
/// - `MintTokens (0)`: Authority to mint new tokens
/// - `FreezeAccount (1)`: Authority to freeze/thaw accounts
/// - `AccountOwner (2)`: Owner of a token account
/// - `CloseAccount (3)`: Authority to close a token account
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityType {
    /// Permission to mint new tokens (on Mint accounts)
    MintTokens = 0,

    /// Permission to freeze/thaw token accounts (on Mint accounts)
    FreezeAccount = 1,

    /// Owner of a token account (on Account accounts)
    AccountOwner = 2,

    /// Permission to close a token account (on Account accounts)
    CloseAccount = 3,
}

impl AuthorityType {
    /// Parse AuthorityType from a single byte.
    ///
    /// # Arguments
    /// * `value` - The byte value to parse
    ///
    /// # Returns
    /// * `Ok(AuthorityType)` - Successfully parsed
    /// * `Err(InvalidInstruction)` - Unknown authority type
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(AuthorityType::MintTokens),
            1 => Ok(AuthorityType::FreezeAccount),
            2 => Ok(AuthorityType::AccountOwner),
            3 => Ok(AuthorityType::CloseAccount),
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }
}

// =============================================================================
// TOKEN INSTRUCTION ENUM
// =============================================================================

/// All instructions supported by the token program.
///
/// Each variant contains the instruction-specific data.
/// Account requirements are documented in comments but not encoded.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenInstruction {
    // =========================================================================
    // INITIALIZATION INSTRUCTIONS
    // =========================================================================

    /// Initialize a new mint (create a new token type).
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | mint | ✓ | | The mint to initialize |
    /// | 1 | rent | | | Rent sysvar |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (0)
    /// [1]: decimals (u8)
    /// [2..34]: mint_authority (Pubkey, 32 bytes)
    /// [34]: freeze_authority_option (0 = None, 1 = Some)
    /// [35..67]: freeze_authority (Pubkey, 32 bytes, if option = 1)
    /// ```
    ///
    /// # Example
    ///
    /// Creating a token with 6 decimals (like USDC):
    /// - 1 USDC = 1,000,000 base units
    /// - Display: amount / 10^6
    InitializeMint {
        /// Number of decimals for display purposes
        decimals: u8,

        /// Authority that can mint new tokens
        mint_authority: Pubkey,

        /// Optional authority that can freeze token accounts
        freeze_authority: Option<Pubkey>,
    },

    /// Initialize a new token account.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | account | ✓ | | The account to initialize |
    /// | 1 | mint | | | The mint this account holds |
    /// | 2 | owner | | | The owner of this account |
    /// | 3 | rent | | | Rent sysvar |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (1)
    /// ```
    ///
    /// No additional data - all info comes from accounts.
    InitializeAccount,

    /// Initialize a multisig authority.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | multisig | ✓ | | The multisig to initialize |
    /// | 1 | rent | | | Rent sysvar |
    /// | 2..2+N | signers | | | The N signer pubkeys |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (2)
    /// [1]: m (required signatures)
    /// ```
    ///
    /// # Constraints
    ///
    /// - 1 ≤ M ≤ N ≤ 11
    /// - Each signer must be unique
    InitializeMultisig {
        /// Number of required signatures (M in M-of-N)
        m: u8,
    },

    // =========================================================================
    // TOKEN OPERATIONS
    // =========================================================================

    /// Transfer tokens from one account to another.
    ///
    /// # Account Requirements (Single Authority)
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | source | ✓ | | Source token account |
    /// | 1 | destination | ✓ | | Destination token account |
    /// | 2 | authority | | ✓ | Owner or delegate |
    ///
    /// # Account Requirements (Multisig Authority)
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | source | ✓ | | Source token account |
    /// | 1 | destination | ✓ | | Destination token account |
    /// | 2 | multisig | | | Multisig authority |
    /// | 3..3+M | signers | | ✓ | M signer accounts |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (3)
    /// [1..9]: amount (u64, little-endian)
    /// ```
    Transfer {
        /// Amount of tokens to transfer
        amount: u64,
    },

    /// Approve a delegate to transfer tokens.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | source | ✓ | | Token account to approve from |
    /// | 1 | delegate | | | The delegate to approve |
    /// | 2 | owner | | ✓ | Token account owner |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (4)
    /// [1..9]: amount (u64, little-endian)
    /// ```
    ///
    /// # Notes
    ///
    /// - Replaces any existing delegate
    /// - Amount is the MAXIMUM the delegate can transfer
    /// - Use Revoke to remove the delegate
    Approve {
        /// Maximum amount delegate can transfer
        amount: u64,
    },

    /// Revoke a delegate's approval.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | source | ✓ | | Token account |
    /// | 1 | owner | | ✓ | Token account owner |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (5)
    /// ```
    Revoke,

    /// Change an authority on a mint or token account.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | account | ✓ | | Mint or token account |
    /// | 1 | current_authority | | ✓ | Current authority |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (6)
    /// [1]: authority_type (u8)
    /// [2]: new_authority_option (0 = None, 1 = Some)
    /// [3..35]: new_authority (Pubkey, 32 bytes, if option = 1)
    /// ```
    ///
    /// # Notes
    ///
    /// - Setting to None is PERMANENT for MintTokens and FreezeAccount
    /// - Cannot change AccountOwner to None
    SetAuthority {
        /// Which authority to change
        authority_type: AuthorityType,

        /// New authority (None to remove permanently)
        new_authority: Option<Pubkey>,
    },

    /// Mint new tokens to an account.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | mint | ✓ | | The mint |
    /// | 1 | destination | ✓ | | Account to mint to |
    /// | 2 | mint_authority | | ✓ | Mint authority |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (7)
    /// [1..9]: amount (u64, little-endian)
    /// ```
    MintTo {
        /// Amount of tokens to mint
        amount: u64,
    },

    /// Burn tokens from an account.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | account | ✓ | | Account to burn from |
    /// | 1 | mint | ✓ | | The mint |
    /// | 2 | authority | | ✓ | Owner or delegate |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (8)
    /// [1..9]: amount (u64, little-endian)
    /// ```
    Burn {
        /// Amount of tokens to burn
        amount: u64,
    },

    /// Close a token account and reclaim rent.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | account | ✓ | | Account to close |
    /// | 1 | destination | ✓ | | Receives the rent lamports |
    /// | 2 | authority | | ✓ | Close authority or owner |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (9)
    /// ```
    ///
    /// # Constraints
    ///
    /// - Token balance must be 0
    /// - For native (wrapped SOL), all lamports transferred
    CloseAccount,

    /// Freeze a token account (prevent transfers).
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | account | ✓ | | Account to freeze |
    /// | 1 | mint | | | The mint |
    /// | 2 | freeze_authority | | ✓ | Freeze authority |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (10)
    /// ```
    FreezeAccount,

    /// Thaw a frozen token account.
    ///
    /// # Account Requirements
    ///
    /// | # | Account | Writable | Signer | Description |
    /// |---|---------|----------|--------|-------------|
    /// | 0 | account | ✓ | | Account to thaw |
    /// | 1 | mint | | | The mint |
    /// | 2 | freeze_authority | | ✓ | Freeze authority |
    ///
    /// # Data Layout
    ///
    /// ```text
    /// [0]: discriminant (11)
    /// ```
    ThawAccount,
}

// =============================================================================
// INSTRUCTION PARSING (UNPACK)
// =============================================================================

impl TokenInstruction {
    /// Parse instruction data into a TokenInstruction.
    ///
    /// # Arguments
    /// * `input` - Raw instruction data bytes
    ///
    /// # Returns
    /// * `Ok(TokenInstruction)` - Successfully parsed instruction
    /// * `Err(InvalidInstruction)` - Could not parse
    ///
    /// # Format
    ///
    /// First byte is the discriminant, remaining bytes are instruction-specific.
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Get the discriminant (first byte)
        let (&discriminant, rest) = input
            .split_first()
            .ok_or(TokenError::InvalidInstruction)?;

        // Parse based on discriminant
        Ok(match discriminant {
            // =================================================================
            // 0: InitializeMint
            // =================================================================
            0 => {
                // Need at least: decimals(1) + mint_authority(32) + option(1) = 34 bytes
                if rest.len() < 34 {
                    return Err(TokenError::InvalidInstruction.into());
                }

                let decimals = rest[0];

                // Parse mint_authority (bytes 1-32)
                let mint_authority = Pubkey::new_from_array(
                    rest[1..33]
                        .try_into()
                        .map_err(|_| TokenError::InvalidInstruction)?,
                );

                // Parse freeze_authority option
                let freeze_authority = if rest[33] == 1 {
                    // Has freeze authority - need 32 more bytes
                    if rest.len() < 66 {
                        return Err(TokenError::InvalidInstruction.into());
                    }
                    Some(Pubkey::new_from_array(
                        rest[34..66]
                            .try_into()
                            .map_err(|_| TokenError::InvalidInstruction)?,
                    ))
                } else if rest[33] == 0 {
                    None
                } else {
                    return Err(TokenError::InvalidInstruction.into());
                };

                TokenInstruction::InitializeMint {
                    decimals,
                    mint_authority,
                    freeze_authority,
                }
            }

            // =================================================================
            // 1: InitializeAccount
            // =================================================================
            1 => TokenInstruction::InitializeAccount,

            // =================================================================
            // 2: InitializeMultisig
            // =================================================================
            2 => {
                if rest.is_empty() {
                    return Err(TokenError::InvalidInstruction.into());
                }
                TokenInstruction::InitializeMultisig { m: rest[0] }
            }

            // =================================================================
            // 3: Transfer
            // =================================================================
            3 => {
                if rest.len() < 8 {
                    return Err(TokenError::InvalidInstruction.into());
                }
                let amount = u64::from_le_bytes(
                    rest[..8]
                        .try_into()
                        .map_err(|_| TokenError::InvalidInstruction)?,
                );
                TokenInstruction::Transfer { amount }
            }

            // =================================================================
            // 4: Approve
            // =================================================================
            4 => {
                if rest.len() < 8 {
                    return Err(TokenError::InvalidInstruction.into());
                }
                let amount = u64::from_le_bytes(
                    rest[..8]
                        .try_into()
                        .map_err(|_| TokenError::InvalidInstruction)?,
                );
                TokenInstruction::Approve { amount }
            }

            // =================================================================
            // 5: Revoke
            // =================================================================
            5 => TokenInstruction::Revoke,

            // =================================================================
            // 6: SetAuthority
            // =================================================================
            6 => {
                if rest.len() < 2 {
                    return Err(TokenError::InvalidInstruction.into());
                }

                let authority_type = AuthorityType::from_u8(rest[0])?;

                let new_authority = if rest[1] == 1 {
                    if rest.len() < 34 {
                        return Err(TokenError::InvalidInstruction.into());
                    }
                    Some(Pubkey::new_from_array(
                        rest[2..34]
                            .try_into()
                            .map_err(|_| TokenError::InvalidInstruction)?,
                    ))
                } else if rest[1] == 0 {
                    None
                } else {
                    return Err(TokenError::InvalidInstruction.into());
                };

                TokenInstruction::SetAuthority {
                    authority_type,
                    new_authority,
                }
            }

            // =================================================================
            // 7: MintTo
            // =================================================================
            7 => {
                if rest.len() < 8 {
                    return Err(TokenError::InvalidInstruction.into());
                }
                let amount = u64::from_le_bytes(
                    rest[..8]
                        .try_into()
                        .map_err(|_| TokenError::InvalidInstruction)?,
                );
                TokenInstruction::MintTo { amount }
            }

            // =================================================================
            // 8: Burn
            // =================================================================
            8 => {
                if rest.len() < 8 {
                    return Err(TokenError::InvalidInstruction.into());
                }
                let amount = u64::from_le_bytes(
                    rest[..8]
                        .try_into()
                        .map_err(|_| TokenError::InvalidInstruction)?,
                );
                TokenInstruction::Burn { amount }
            }

            // =================================================================
            // 9: CloseAccount
            // =================================================================
            9 => TokenInstruction::CloseAccount,

            // =================================================================
            // 10: FreezeAccount
            // =================================================================
            10 => TokenInstruction::FreezeAccount,

            // =================================================================
            // 11: ThawAccount
            // =================================================================
            11 => TokenInstruction::ThawAccount,

            // =================================================================
            // Unknown instruction
            // =================================================================
            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }

    // =========================================================================
    // INSTRUCTION PACKING (for tests and clients)
    // =========================================================================

    /// Pack instruction into bytes.
    ///
    /// This is the inverse of `unpack()`.
    /// Used by tests and client libraries to create instruction data.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        match self {
            TokenInstruction::InitializeMint {
                decimals,
                mint_authority,
                freeze_authority,
            } => {
                buf.push(0); // discriminant
                buf.push(*decimals);
                buf.extend_from_slice(mint_authority.as_ref());
                match freeze_authority {
                    Some(authority) => {
                        buf.push(1); // Some
                        buf.extend_from_slice(authority.as_ref());
                    }
                    None => {
                        buf.push(0); // None
                    }
                }
            }

            TokenInstruction::InitializeAccount => {
                buf.push(1);
            }

            TokenInstruction::InitializeMultisig { m } => {
                buf.push(2);
                buf.push(*m);
            }

            TokenInstruction::Transfer { amount } => {
                buf.push(3);
                buf.extend_from_slice(&amount.to_le_bytes());
            }

            TokenInstruction::Approve { amount } => {
                buf.push(4);
                buf.extend_from_slice(&amount.to_le_bytes());
            }

            TokenInstruction::Revoke => {
                buf.push(5);
            }

            TokenInstruction::SetAuthority {
                authority_type,
                new_authority,
            } => {
                buf.push(6);
                buf.push(*authority_type as u8);
                match new_authority {
                    Some(authority) => {
                        buf.push(1);
                        buf.extend_from_slice(authority.as_ref());
                    }
                    None => {
                        buf.push(0);
                    }
                }
            }

            TokenInstruction::MintTo { amount } => {
                buf.push(7);
                buf.extend_from_slice(&amount.to_le_bytes());
            }

            TokenInstruction::Burn { amount } => {
                buf.push(8);
                buf.extend_from_slice(&amount.to_le_bytes());
            }

            TokenInstruction::CloseAccount => {
                buf.push(9);
            }

            TokenInstruction::FreezeAccount => {
                buf.push(10);
            }

            TokenInstruction::ThawAccount => {
                buf.push(11);
            }
        }

        buf
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

INSTRUCTION FORMAT
==================

Every instruction sent to a Solana program has:
1. Program ID (which program to call)
2. Accounts (which accounts are involved)
3. Data (instruction-specific bytes)

This module defines how we parse #3 (the data).

Our format:
[discriminant: 1 byte][instruction_data: varies]

The discriminant tells us which instruction it is.
The remaining bytes depend on the instruction.

WHY NOT USE BORSH?
==================

Borsh is a serialization format commonly used with Anchor.
SPL Token uses manual serialization because:

1. Stability: Exact byte layout that never changes
2. Compatibility: Must match existing SPL Token clients
3. Efficiency: No overhead from generic serialization
4. Control: We know exactly what bytes mean

Manual parsing also teaches you what's really happening!

LITTLE-ENDIAN
=============

u64::from_le_bytes([...])

"Little-endian" means least significant byte first.

Example: 1000 (0x3E8) in little-endian:
[0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]

Why little-endian?
- x86/ARM processors are little-endian
- Matches what Solana uses internally
- More efficient (no byte swapping needed)

OPTION ENCODING
===============

We encode Option<Pubkey> as:
- 0 = None
- 1 = Some, followed by 32 bytes

This is simpler than Borsh's Option encoding and matches SPL Token.

SPLIT_FIRST
===========

let (&discriminant, rest) = input.split_first()?;

This splits: [0x03, 0xE8, 0x03, ...]
Into: discriminant = 0x03, rest = [0xE8, 0x03, ...]

If input is empty, returns None (we convert to error).

TRY_INTO FOR ARRAYS
===================

rest[..8].try_into()

Converts a slice to a fixed-size array.
[0xE8, 0x03, 0x00, ...] (slice) -> [u8; 8] (array)

Required because from_le_bytes needs exactly 8 bytes.

THE PACK METHOD
===============

pack() is the inverse of unpack().
- unpack: bytes -> TokenInstruction
- pack: TokenInstruction -> bytes

Used for:
- Tests (create instruction data)
- Client libraries (build transactions)
- Debugging (verify parsing)

Example test:
let original = TokenInstruction::Transfer { amount: 1000 };
let bytes = original.pack();
let parsed = TokenInstruction::unpack(&bytes).unwrap();
assert_eq!(original, parsed);
*/