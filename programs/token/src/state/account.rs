//! Token Account State
//!
//! A Token Account holds tokens for a specific owner.
//! It's like a wallet specifically for ONE token type.
//!
//! # Real World Analogy
//!
//! - Mint = Currency definition (e.g., "US Dollar")
//! - Token Account = Bank account for that currency
//!
//! Each user needs a separate Token Account for each token they hold.
//!
//! # Example
//!
//! Alice wants to hold USDC and BONK:
//! - She needs 1 Token Account linked to USDC mint
//! - She needs 1 Token Account linked to BONK mint
//! - Total: 2 Token Accounts
//!
//! # Size: 165 bytes (matches SPL Token exactly)

use crate::error::TokenError;
use crate::state::{COption, Pack};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

// =============================================================================
// ACCOUNT STATE ENUM
// =============================================================================

/// The state of a token account.
///
/// Represents the lifecycle of a token account.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccountState {
    /// Account is not yet initialized.
    ///
    /// This is the default state when an account is created.
    /// The account contains garbage/zeros and should not be used.
    #[default]
    Uninitialized,

    /// Account is initialized and active.
    ///
    /// Normal operating state. Can transfer, receive, burn, etc.
    Initialized,

    /// Account is frozen by the freeze authority.
    ///
    /// Cannot transfer tokens OUT.
    /// CAN still receive tokens.
    /// Can be thawed by freeze_authority.
    Frozen,
}

impl AccountState {
    /// Convert a u8 byte to AccountState.
    ///
    /// # Values
    ///
    /// - 0 = Uninitialized
    /// - 1 = Initialized
    /// - 2 = Frozen
    /// - Other = Error
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(AccountState::Uninitialized),
            1 => Ok(AccountState::Initialized),
            2 => Ok(AccountState::Frozen),
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }

    /// Convert AccountState to a u8 byte.
    pub fn to_u8(self) -> u8 {
        match self {
            AccountState::Uninitialized => 0,
            AccountState::Initialized => 1,
            AccountState::Frozen => 2,
        }
    }
}

// =============================================================================
// TOKEN ACCOUNT STRUCTURE
// =============================================================================

/// Token account data structure.
///
/// Holds tokens of a specific mint for a specific owner.
///
/// # Memory Layout (165 bytes total)
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │ Offset │ Size │ Field            │ Type                        │
/// ├────────┼──────┼──────────────────┼─────────────────────────────┤
/// │ 0      │ 32   │ mint             │ Pubkey                      │
/// │ 32     │ 32   │ owner            │ Pubkey                      │
/// │ 64     │ 8    │ amount           │ u64                         │
/// │ 72     │ 36   │ delegate         │ COption<Pubkey>             │
/// │ 108    │ 1    │ state            │ AccountState (u8)           │
/// │ 109    │ 12   │ is_native        │ COption<u64>                │
/// │ 121    │ 8    │ delegated_amount │ u64                         │
/// │ 129    │ 36   │ close_authority  │ COption<Pubkey>             │
/// ├────────┼──────┼──────────────────┼─────────────────────────────┤
/// │ Total  │ 165  │                  │                             │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Example Usage
///
/// ```ignore
/// // Reading a token account
/// let data = token_account_info.data.borrow();
/// let account = Account::unpack_from_slice(&data)?;
///
/// // Check balance
/// println!("Balance: {}", account.amount);
///
/// // Check owner
/// if account.owner != expected_owner {
///     return Err(TokenError::OwnerMismatch.into());
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// The mint this account holds tokens of.
    ///
    /// Every token account is linked to exactly ONE mint.
    /// You cannot change this after initialization.
    ///
    /// # Validation
    ///
    /// When transferring, both source and destination must have same mint.
    /// ```ignore
    /// if source.mint != destination.mint {
    ///     return Err(TokenError::MintMismatch.into());
    /// }
    /// ```
    pub mint: Pubkey,

    /// The owner of this token account.
    ///
    /// The owner can:
    /// - Transfer tokens from this account
    /// - Approve delegates
    /// - Revoke delegates
    /// - Close the account (if balance is 0)
    ///
    /// # Common Patterns
    ///
    /// - User wallet: owner = user's wallet pubkey
    /// - Protocol vault: owner = PDA (program controls it)
    /// - Escrow: owner = escrow PDA
    ///
    /// # Changing Owner
    ///
    /// Use SetAuthority with AuthorityType::AccountOwner.
    /// New owner must sign the transaction.
    pub owner: Pubkey,

    /// The number of tokens in this account.
    ///
    /// Always in base units (not display units).
    ///
    /// # Example
    ///
    /// If mint has decimals = 6:
    /// - amount = 1_000_000 means "1.0 tokens"
    /// - amount = 1_500_000 means "1.5 tokens"
    ///
    /// # Operations
    ///
    /// - Increases on: Transfer (receiving), MintTo
    /// - Decreases on: Transfer (sending), Burn
    ///
    /// # Invariant
    ///
    /// amount <= delegated_amount when delegate is using allowance
    pub amount: u64,

    /// Optional delegate who can transfer tokens on owner's behalf.
    ///
    /// Like "approve" in ERC-20.
    ///
    /// # Workflow
    ///
    /// 1. Owner calls Approve(delegate, amount)
    /// 2. delegate can now Transfer up to amount
    /// 3. Each Transfer decreases delegated_amount
    /// 4. Owner can Revoke anytime
    ///
    /// # Values
    ///
    /// - None: No delegate approved
    /// - Some(pubkey): That pubkey can transfer up to delegated_amount
    ///
    /// # Use Cases
    ///
    /// - DEX: Approve DEX to spend your tokens for swaps
    /// - Subscriptions: Approve service to pull payments
    /// - Games: Approve game to move your items
    pub delegate: COption<Pubkey>,

    /// The state of this account.
    ///
    /// See AccountState enum for details.
    pub state: AccountState,

    /// If Some, this is a "native" (wrapped SOL) account.
    ///
    /// # What is Native/Wrapped SOL?
    ///
    /// SOL is not an SPL token - it's the native currency.
    /// But sometimes you want to treat SOL like a token.
    ///
    /// Wrapped SOL:
    /// 1. Create a token account for the native mint
    /// 2. Transfer SOL into it (becomes wrapped)
    /// 3. Now it acts like any other SPL token
    /// 4. Close the account to unwrap back to SOL
    ///
    /// # Values
    ///
    /// - None: Regular token account (not native)
    /// - Some(rent_exempt_reserve): Native SOL account
    ///
    /// The value stores the rent-exempt reserve amount.
    /// Actual SOL balance = lamports - rent_exempt_reserve
    ///
    /// # Note
    ///
    /// We won't fully implement native tokens in this tutorial,
    /// but the field must be present for compatibility.
    pub is_native: COption<u64>,

    /// Amount currently approved for the delegate.
    ///
    /// # Behavior
    ///
    /// - Set by Approve instruction
    /// - Decreases each time delegate transfers
    /// - Reset to 0 by Revoke
    /// - When it hits 0, delegate is automatically cleared
    ///
    /// # Example
    ///
    /// 1. Owner approves delegate for 1000 tokens
    ///    - delegate = Some(delegate_pubkey)
    ///    - delegated_amount = 1000
    ///
    /// 2. Delegate transfers 400 tokens
    ///    - delegated_amount = 600
    ///
    /// 3. Delegate transfers 600 tokens
    ///    - delegated_amount = 0
    ///    - delegate = None (automatically cleared)
    pub delegated_amount: u64,

    /// Optional authority who can close this account.
    ///
    /// # Values
    ///
    /// - None: Owner is the close authority (default)
    /// - Some(pubkey): That pubkey can close the account
    ///
    /// # Why Separate Close Authority?
    ///
    /// Sometimes you want someone else to reclaim rent.
    ///
    /// Example: Protocol creates accounts for users
    /// - owner = user (they control the tokens)
    /// - close_authority = protocol (they reclaim rent when done)
    ///
    /// # Closing Requirements
    ///
    /// - Token balance must be 0
    /// - Close authority must sign
    /// - Rent lamports go to specified destination
    pub close_authority: COption<Pubkey>,
}

// =============================================================================
// ASSOCIATED CONSTANTS AND METHODS
// =============================================================================

impl Account {
    /// Size of Account when serialized.
    ///
    /// Calculation:
    /// - mint: 32 bytes
    /// - owner: 32 bytes
    /// - amount: 8 bytes
    /// - delegate: 36 bytes (COption<Pubkey>)
    /// - state: 1 byte
    /// - is_native: 12 bytes (COption<u64>)
    /// - delegated_amount: 8 bytes
    /// - close_authority: 36 bytes (COption<Pubkey>)
    /// - Total: 32 + 32 + 8 + 36 + 1 + 12 + 8 + 36 = 165 bytes
    pub const LEN: usize = 165;

    /// Check if the account is frozen.
    ///
    /// Frozen accounts cannot transfer tokens out.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if account.is_frozen() {
    ///     return Err(TokenError::AccountFrozen.into());
    /// }
    /// ```
    pub fn is_frozen(&self) -> bool {
        self.state == AccountState::Frozen
    }

    /// Check if the account is initialized.
    ///
    /// Uninitialized accounts should not be used.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if !account.is_initialized() {
    ///     return Err(TokenError::UninitializedAccount.into());
    /// }
    /// ```
    pub fn is_initialized(&self) -> bool {
        self.state != AccountState::Uninitialized
    }

    /// Check if this is a native (wrapped SOL) account.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if account.is_native() {
    ///     // Special handling for wrapped SOL
    /// }
    /// ```
    pub fn is_native(&self) -> bool {
        self.is_native.is_some()
    }
}

// =============================================================================
// PACK TRAIT IMPLEMENTATION
// =============================================================================

impl Pack for Account {
    const LEN: usize = 165;

    /// Deserialize an Account from bytes.
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Create fixed-size reference
        let input = array_ref![input, 0, Account::LEN];

        // Split into fields
        // Sizes: 32 + 32 + 8 + 36 + 1 + 12 + 8 + 36 = 165
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            mint,
            owner,
            amount,
            delegate,
            state,
            is_native,
            delegated_amount,
            close_authority,
        ) = array_refs![input, 32, 32, 8, 36, 1, 12, 8, 36];

        // Parse each field
        Ok(Account {
            mint: Pubkey::new_from_array(*mint),
            owner: Pubkey::new_from_array(*owner),
            amount: u64::from_le_bytes(*amount),
            delegate: unpack_coption_pubkey(delegate)?,
            state: AccountState::from_u8(state[0])?,
            is_native: unpack_coption_u64(is_native)?,
            delegated_amount: u64::from_le_bytes(*delegated_amount),
            close_authority: unpack_coption_pubkey(close_authority)?,
        })
    }

    /// Serialize an Account to bytes.
    fn pack(&self, output: &mut [u8]) -> Result<(), ProgramError> {
        // Create fixed-size mutable reference
        let output = array_mut_ref![output, 0, Account::LEN];

        // Split into field destinations
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            mint_dst,
            owner_dst,
            amount_dst,
            delegate_dst,
            state_dst,
            is_native_dst,
            delegated_amount_dst,
            close_authority_dst,
        ) = mut_array_refs![output, 32, 32, 8, 36, 1, 12, 8, 36];

        // Write each field
        mint_dst.copy_from_slice(self.mint.as_ref());
        owner_dst.copy_from_slice(self.owner.as_ref());
        *amount_dst = self.amount.to_le_bytes();
        pack_coption_pubkey(&self.delegate, delegate_dst);
        state_dst[0] = self.state.to_u8();
        pack_coption_u64(&self.is_native, is_native_dst);
        *delegated_amount_dst = self.delegated_amount.to_le_bytes();
        pack_coption_pubkey(&self.close_authority, close_authority_dst);

        Ok(())
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR COPTION<PUBKEY>
// =============================================================================

/// Unpack COption<Pubkey> from 36 bytes.
///
/// Layout: [tag: 4 bytes][pubkey: 32 bytes]
fn unpack_coption_pubkey(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];

    match u32::from_le_bytes(*tag) {
        0 => Ok(COption::none()),
        1 => Ok(COption::some(Pubkey::new_from_array(*body))),
        _ => Err(TokenError::InvalidInstruction.into()),
    }
}

/// Pack COption<Pubkey> into 36 bytes.
fn pack_coption_pubkey(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    let (tag, body) = mut_array_refs![dst, 4, 32];

    match src.as_ref() {
        Some(pubkey) => {
            *tag = 1u32.to_le_bytes();
            body.copy_from_slice(pubkey.as_ref());
        }
        None => {
            *tag = 0u32.to_le_bytes();
            body.fill(0);
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR COPTION<U64>
// =============================================================================

/// Unpack COption<u64> from 12 bytes.
///
/// Layout: [tag: 4 bytes][value: 8 bytes]
///
/// Used for the is_native field (wrapped SOL tracking).
fn unpack_coption_u64(src: &[u8; 12]) -> Result<COption<u64>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 8];

    match u32::from_le_bytes(*tag) {
        0 => Ok(COption::none()),
        1 => Ok(COption::some(u64::from_le_bytes(*body))),
        _ => Err(TokenError::InvalidInstruction.into()),
    }
}

/// Pack COption<u64> into 12 bytes.
fn pack_coption_u64(src: &COption<u64>, dst: &mut [u8; 12]) {
    let (tag, body) = mut_array_refs![dst, 4, 8];

    match src.as_ref() {
        Some(value) => {
            *tag = 1u32.to_le_bytes();
            *body = value.to_le_bytes();
        }
        None => {
            *tag = 0u32.to_le_bytes();
            body.fill(0);
        }
    }
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test roundtrip pack/unpack.
    #[test]
    fn test_account_pack_unpack_roundtrip() {
        let original = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 1_000_000_000,
            delegate: COption::some(Pubkey::new_unique()),
            state: AccountState::Initialized,
            is_native: COption::none(),
            delegated_amount: 500_000_000,
            close_authority: COption::some(Pubkey::new_unique()),
        };

        let mut packed = [0u8; Account::LEN];
        original.pack(&mut packed).unwrap();

        let unpacked = Account::unpack(&packed).unwrap();

        assert_eq!(original, unpacked);
    }

    /// Test account with no delegate.
    #[test]
    fn test_account_no_delegate() {
        let account = Account {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 100,
            delegate: COption::none(),
            state: AccountState::Initialized,
            is_native: COption::none(),
            delegated_amount: 0,
            close_authority: COption::none(),
        };

        let mut packed = [0u8; Account::LEN];
        account.pack(&mut packed).unwrap();

        let unpacked = Account::unpack(&packed).unwrap();

        assert!(unpacked.delegate.is_none());
        assert_eq!(unpacked.delegated_amount, 0);
    }

    /// Test frozen account state.
    #[test]
    fn test_account_frozen() {
        let mut account = Account::default();
        account.state = AccountState::Frozen;

        assert!(account.is_frozen());
        assert!(account.is_initialized()); // Frozen is still initialized
    }

    /// Test uninitialized account state.
    #[test]
    fn test_account_uninitialized() {
        let account = Account::default();

        assert!(!account.is_initialized());
        assert!(!account.is_frozen());
    }

    /// Test native account detection.
    #[test]
    fn test_account_native() {
        let mut account = Account::default();

        assert!(!account.is_native());

        account.is_native = COption::some(890880); // Rent exempt amount

        assert!(account.is_native());
    }

    /// Test size is correct.
    #[test]
    fn test_account_size() {
        assert_eq!(Account::LEN, 165);
    }

    /// Test AccountState conversion.
    #[test]
    fn test_account_state_conversion() {
        assert_eq!(AccountState::from_u8(0).unwrap(), AccountState::Uninitialized);
        assert_eq!(AccountState::from_u8(1).unwrap(), AccountState::Initialized);
        assert_eq!(AccountState::from_u8(2).unwrap(), AccountState::Frozen);
        assert!(AccountState::from_u8(3).is_err());

        assert_eq!(AccountState::Uninitialized.to_u8(), 0);
        assert_eq!(AccountState::Initialized.to_u8(), 1);
        assert_eq!(AccountState::Frozen.to_u8(), 2);
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHAT IS A TOKEN ACCOUNT?
========================

Think of it like a bank account:
- Bank account holds one currency (USD, EUR, etc.)
- Token account holds one token type (USDC, BONK, etc.)

One person can have many token accounts:
- One for USDC
- One for BONK
- One for each NFT they own

FIELD EXPLANATIONS
==================

1. mint (Pubkey, 32 bytes)
   
   Links this account to a specific token type.
   
   Example:
   - USDC mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
   - Your USDC account has mint = that pubkey
   
   Cannot be changed after creation.

2. owner (Pubkey, 32 bytes)
   
   Who controls this account.
   
   The owner can:
   - Transfer tokens away
   - Approve delegates
   - Close the account
   
   Can be changed with SetAuthority.

3. amount (u64, 8 bytes)
   
   Current token balance.
   
   Always in "base units":
   - If decimals = 6: amount 1000000 = "1.0 tokens"
   - If decimals = 9: amount 1000000000 = "1.0 tokens"

4. delegate + delegated_amount
   
   Allowance system (like ERC-20 approve).
   
   Flow:
   - Owner: Approve(delegate, 1000)
   - delegate can now Transfer up to 1000
   - Each transfer decreases delegated_amount
   - When 0, delegate is cleared

5. state (AccountState, 1 byte)
   
   Lifecycle state:
   - Uninitialized: Fresh account, garbage data
   - Initialized: Normal, can transfer
   - Frozen: Locked, cannot transfer out

6. is_native (COption<u64>, 12 bytes)
   
   For wrapped SOL accounts.
   
   Regular accounts: None
   Wrapped SOL: Some(rent_exempt_lamports)

7. close_authority (COption<Pubkey>, 36 bytes)
   
   Who can close this account.
   
   If None: owner is close authority
   If Some(x): x can close (and reclaim rent)

ASSOCIATED TOKEN ACCOUNTS
=========================

In practice, most token accounts are "Associated Token Accounts" (ATAs).

ATA = deterministic address derived from:
- Owner wallet
- Mint
- Token program

This means:
- Given a wallet and mint, you know the token account address
- No need to store or look it up
- Standard across the ecosystem

We're not implementing ATAs here (that's a separate program),
but this Account struct is what ATAs contain.

SIZE BREAKDOWN
==============

32 (mint)
+ 32 (owner)  
+ 8 (amount)
+ 36 (delegate: 4 tag + 32 pubkey)
+ 1 (state)
+ 12 (is_native: 4 tag + 8 u64)
+ 8 (delegated_amount)
+ 36 (close_authority: 4 tag + 32 pubkey)
= 165 bytes

DELEGATE SYSTEM
===============

Similar to ERC-20's approve/transferFrom.

Why use delegates?
1. DEXes need to take tokens during swaps
2. Subscription services need to pull payments
3. Games need to move items

Flow:
1. User approves DEX for 100 USDC
2. User submits swap order
3. DEX calls Transfer using delegate authority
4. Tokens move from user to DEX
5. User gets swap output

Safety:
- Delegate can only spend up to delegated_amount
- Owner can Revoke anytime
- Delegate is cleared when amount reaches 0

FROZEN ACCOUNTS
===============

When freeze_authority freezes an account:
- state changes to Frozen
- Account cannot transfer OUT
- Account CAN still receive tokens
- Only freeze_authority can thaw

Use cases:
- Compliance: Freeze suspicious accounts
- Security: Freeze compromised accounts
- Games: Lock items during gameplay
*/