//! Mint Account State
//!
//! A Mint defines a token type. Think of it like a currency definition.
//!
//! # Real World Examples
//!
//! - USDC has one Mint account that defines it
//! - BONK has one Mint account
//! - Each NFT in a collection has its own Mint
//! - Your custom token will have one Mint
//!
//! # What a Mint Controls
//!
//! 1. Who can create new tokens (mint_authority)
//! 2. Total tokens in existence (supply)
//! 3. How to display amounts (decimals)
//! 4. Who can freeze accounts (freeze_authority)
//!
//! # Size: 82 bytes (matches SPL Token exactly)

use crate::error::TokenError;
use crate::state::{COption, Pack};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

// =============================================================================
// MINT STRUCTURE
// =============================================================================

/// Mint account data structure.
///
/// This is the core structure that defines a token type.
/// Every token account (Account struct) references exactly one Mint.
///
/// # Memory Layout (82 bytes total)
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │ Offset │ Size │ Field            │ Type                        │
/// ├────────┼──────┼──────────────────┼─────────────────────────────┤
/// │ 0      │ 36   │ mint_authority   │ COption<Pubkey>             │
/// │ 36     │ 8    │ supply           │ u64                         │
/// │ 44     │ 1    │ decimals         │ u8                          │
/// │ 45     │ 1    │ is_initialized   │ bool (0 or 1)               │
/// │ 46     │ 36   │ freeze_authority │ COption<Pubkey>             │
/// ├────────┼──────┼──────────────────┼─────────────────────────────┤
/// │ Total  │ 82   │                  │                             │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// # COption<Pubkey> Layout (36 bytes)
///
/// ```text
/// ┌──────────────────────────────────────┐
/// │ Bytes 0-3  │ Tag (u32 little-endian) │
/// │ Bytes 4-35 │ Pubkey (32 bytes)       │
/// └──────────────────────────────────────┘
/// Tag = 0: None (Pubkey bytes are zeros/ignored)
/// Tag = 1: Some(Pubkey)
/// ```
///
/// # Example Usage
///
/// ```ignore
/// // Reading a Mint from an account
/// let mint_data = mint_account_info.data.borrow();
/// let mint = Mint::unpack_from_slice(&mint_data)?;
///
/// // Check if initialized
/// if !mint.is_initialized {
///     return Err(TokenError::UninitializedAccount.into());
/// }
///
/// // Get supply
/// println!("Total supply: {}", mint.supply);
///
/// // Check mint authority
/// if let Some(authority) = mint.mint_authority.as_ref() {
///     println!("Mint authority: {}", authority);
/// } else {
///     println!("Fixed supply - no mint authority");
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mint {
    /// The authority that can mint new tokens.
    ///
    /// # Values
    ///
    /// - `Some(pubkey)`: That pubkey can call MintTo instruction
    /// - `None`: No more tokens can EVER be minted (fixed supply forever)
    ///
    /// # Security Notes
    ///
    /// 1. Setting this to None is PERMANENT and IRREVERSIBLE
    /// 2. Use SetAuthority with new_authority = None to disable minting
    /// 3. For protocols, this is often a PDA so the program controls minting
    ///
    /// # Common Patterns
    ///
    /// ```text
    /// Fixed Supply Token:
    ///   1. Create mint with your keypair as authority
    ///   2. Mint all tokens you want
    ///   3. SetAuthority to None (permanent)
    ///
    /// Protocol-Controlled Token:
    ///   1. Create mint with PDA as authority
    ///   2. Program mints via CPI when conditions are met
    ///
    /// Admin-Controlled Token:
    ///   1. Create mint with admin keypair
    ///   2. Admin can mint anytime (centralized)
    /// ```
    pub mint_authority: COption<Pubkey>,

    /// Total number of tokens currently in existence.
    ///
    /// # Behavior
    ///
    /// - Increases when MintTo is called
    /// - Decreases when Burn is called
    /// - Never goes negative (would error first)
    ///
    /// # Invariant (must always be true)
    ///
    /// ```text
    /// supply == SUM(all token accounts for this mint).amount
    /// ```
    ///
    /// If this invariant is violated, there's a bug!
    ///
    /// # Limits
    ///
    /// - Maximum: u64::MAX = 18,446,744,073,709,551,615
    /// - With 9 decimals: ~18.4 billion whole tokens
    /// - With 6 decimals: ~18.4 trillion whole tokens
    ///
    /// # Example
    ///
    /// ```text
    /// Mint has supply = 1_000_000_000_000 (1 trillion base units)
    /// Mint has decimals = 6
    /// Display: 1,000,000.000000 tokens (1 million with 6 decimal places)
    /// ```
    pub supply: u64,

    /// Number of decimal places for display purposes.
    ///
    /// # IMPORTANT
    ///
    /// This is ONLY for display/UI purposes!
    /// All on-chain math uses base units (integers).
    ///
    /// # Common Values
    ///
    /// ```text
    /// ┌──────────┬─────────┬────────────────────────────────┐
    /// │ Decimals │ Example │ 1 whole token =                │
    /// ├──────────┼─────────┼────────────────────────────────┤
    /// │ 0        │ NFTs    │ 1 base unit                    │
    /// │ 2        │ Cents   │ 100 base units                 │
    /// │ 6        │ USDC    │ 1,000,000 base units           │
    /// │ 9        │ SOL     │ 1,000,000,000 base units       │
    /// │ 18       │ ETH     │ 1,000,000,000,000,000,000      │
    /// └──────────┴─────────┴────────────────────────────────┘
    /// ```
    ///
    /// # Why Different Decimals?
    ///
    /// - 0 decimals: NFTs (each token is unique, no fractions)
    /// - 6 decimals: Stablecoins (enough precision for dollars)
    /// - 9 decimals: Native tokens (matches SOL precision)
    ///
    /// # Calculation
    ///
    /// ```text
    /// display_amount = base_units / (10 ^ decimals)
    /// base_units = display_amount * (10 ^ decimals)
    ///
    /// Example with 6 decimals:
    /// - User wants to send "1.5 USDC"
    /// - base_units = 1.5 * 1_000_000 = 1_500_000
    /// - Transfer instruction uses amount = 1_500_000
    /// ```
    pub decimals: u8,

    /// Whether this mint has been initialized.
    ///
    /// # States
    ///
    /// - `false`: Account just created, contains garbage data
    /// - `true`: InitializeMint was called, ready to use
    ///
    /// # Security
    ///
    /// ALWAYS check this before using a Mint!
    ///
    /// ```ignore
    /// let mint = Mint::unpack_from_slice(&data)?;
    /// if !mint.is_initialized {
    ///     return Err(TokenError::UninitializedAccount.into());
    /// }
    /// // Now safe to use mint
    /// ```
    ///
    /// # Why This Exists
    ///
    /// When you create an account, Solana fills it with zeros.
    /// But zeros might be valid data (supply = 0, decimals = 0, etc.)
    /// We need a flag to know if InitializeMint was actually called.
    pub is_initialized: bool,

    /// The authority that can freeze/thaw token accounts.
    ///
    /// # Values
    ///
    /// - `Some(pubkey)`: That pubkey can freeze/thaw accounts
    /// - `None`: Freezing is NOT possible for this token (ever)
    ///
    /// # What Freezing Does
    ///
    /// A frozen token account CANNOT:
    /// - Transfer tokens out
    /// - Be closed
    ///
    /// A frozen token account CAN still:
    /// - Receive tokens
    /// - Be thawed by freeze_authority
    ///
    /// # Use Cases
    ///
    /// 1. Stablecoins (USDC, USDT):
    ///    - Freeze accounts for regulatory compliance
    ///    - Freeze accounts suspected of fraud
    ///
    /// 2. Games:
    ///    - Freeze items during active gameplay
    ///    - Prevent trading during tournaments
    ///
    /// 3. Security:
    ///    - Freeze stolen funds before they're moved
    ///
    /// # Important
    ///
    /// Setting to None is PERMANENT.
    /// You cannot add freeze authority later.
    /// Most DeFi tokens set this to None for decentralization.
    pub freeze_authority: COption<Pubkey>,
}

// =============================================================================
// ASSOCIATED CONSTANTS
// =============================================================================

impl Mint {
    /// Size of Mint when serialized to bytes.
    ///
    /// Calculation:
    /// - mint_authority: 36 bytes (4 tag + 32 pubkey)
    /// - supply: 8 bytes (u64)
    /// - decimals: 1 byte (u8)
    /// - is_initialized: 1 byte (bool as u8)
    /// - freeze_authority: 36 bytes (4 tag + 32 pubkey)
    /// - Total: 36 + 8 + 1 + 1 + 36 = 82 bytes
    ///
    /// This matches SPL Token exactly for compatibility.
    pub const LEN: usize = 82;
}

// =============================================================================
// PACK TRAIT IMPLEMENTATION
// =============================================================================

impl Pack for Mint {
    /// The size constant for the Pack trait.
    const LEN: usize = 82;

    /// Deserialize a Mint from a byte slice.
    ///
    /// # Arguments
    ///
    /// * `input` - A byte slice of at least 82 bytes
    ///
    /// # Returns
    ///
    /// * `Ok(Mint)` - Successfully parsed Mint
    /// * `Err(ProgramError)` - Invalid data (bad COption tag, etc.)
    ///
    /// # Panics
    ///
    /// Panics if `input.len() < 82`. Use `unpack_from_slice` for safe parsing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let account_data = mint_account.data.borrow();
    /// let mint = Mint::unpack_from_slice(&account_data)?;
    /// ```
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // =====================================================================
        // STEP 1: Create fixed-size reference
        // =====================================================================
        // array_ref! creates a &[u8; 82] from the input slice
        // This is a compile-time guarantee that we're reading exactly 82 bytes
        // If input is shorter, this will panic (use unpack_from_slice to avoid)
        let input = array_ref![input, 0, Mint::LEN];

        // =====================================================================
        // STEP 2: Split into individual fields
        // =====================================================================
        // array_refs! splits the fixed-size array into smaller fixed-size arrays
        // The sizes MUST sum to the total: 36 + 8 + 1 + 1 + 36 = 82
        //
        // This gives us compile-time bounds checking:
        // - mint_authority_bytes: &[u8; 36]
        // - supply_bytes: &[u8; 8]
        // - decimals_bytes: &[u8; 1]
        // - is_initialized_bytes: &[u8; 1]
        // - freeze_authority_bytes: &[u8; 36]
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            mint_authority_bytes,
            supply_bytes,
            decimals_bytes,
            is_initialized_bytes,
            freeze_authority_bytes,
        ) = array_refs![input, 36, 8, 1, 1, 36];

        // =====================================================================
        // STEP 3: Parse each field
        // =====================================================================

        // Parse mint_authority (COption<Pubkey>)
        // Returns error if tag is not 0 or 1
        let mint_authority = unpack_coption_pubkey(mint_authority_bytes)?;

        // Parse supply (u64, little-endian)
        // from_le_bytes converts [u8; 8] to u64
        // *supply_bytes dereferences &[u8; 8] to [u8; 8]
        let supply = u64::from_le_bytes(*supply_bytes);

        // Parse decimals (u8)
        // Just take the first (only) byte
        let decimals = decimals_bytes[0];

        // Parse is_initialized (bool)
        // 0 = false, anything else = true
        let is_initialized = is_initialized_bytes[0] != 0;

        // Parse freeze_authority (COption<Pubkey>)
        let freeze_authority = unpack_coption_pubkey(freeze_authority_bytes)?;

        // =====================================================================
        // STEP 4: Construct and return Mint
        // =====================================================================
        Ok(Mint {
            mint_authority,
            supply,
            decimals,
            is_initialized,
            freeze_authority,
        })
    }

    /// Serialize a Mint into a byte slice.
    ///
    /// This is the inverse of `unpack()`.
    ///
    /// # Arguments
    ///
    /// * `output` - A mutable byte slice of at least 82 bytes
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully written
    /// * `Err(ProgramError)` - Output too small (shouldn't happen with pack_into_slice)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut account_data = mint_account.data.borrow_mut();
    /// mint.pack_into_slice(&mut account_data)?;
    /// ```
    fn pack(&self, output: &mut [u8]) -> Result<(), ProgramError> {
        // =====================================================================
        // STEP 1: Create fixed-size mutable reference
        // =====================================================================
        let output = array_mut_ref![output, 0, Mint::LEN];

        // =====================================================================
        // STEP 2: Split into individual field destinations
        // =====================================================================
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            mint_authority_dst,
            supply_dst,
            decimals_dst,
            is_initialized_dst,
            freeze_authority_dst,
        ) = mut_array_refs![output, 36, 8, 1, 1, 36];

        // =====================================================================
        // STEP 3: Write each field
        // =====================================================================

        // Write mint_authority
        pack_coption_pubkey(&self.mint_authority, mint_authority_dst);

        // Write supply (u64 to little-endian bytes)
        *supply_dst = self.supply.to_le_bytes();

        // Write decimals
        decimals_dst[0] = self.decimals;

        // Write is_initialized (bool as u8)
        is_initialized_dst[0] = self.is_initialized as u8;

        // Write freeze_authority
        pack_coption_pubkey(&self.freeze_authority, freeze_authority_dst);

        Ok(())
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR COPTION<PUBKEY>
// =============================================================================

/// Unpack a COption<Pubkey> from 36 bytes.
///
/// # Layout
///
/// ```text
/// Bytes 0-3:  Tag (u32, little-endian)
/// Bytes 4-35: Pubkey (32 bytes)
/// ```
///
/// # Tag Values
///
/// - 0: None (Pubkey bytes are ignored)
/// - 1: Some(Pubkey)
/// - Other: Error (InvalidInstruction)
///
/// # Arguments
///
/// * `src` - Reference to exactly 36 bytes
///
/// # Returns
///
/// * `Ok(COption::none())` - Tag was 0
/// * `Ok(COption::some(pubkey))` - Tag was 1
/// * `Err(InvalidInstruction)` - Tag was something else
///
/// # Example
///
/// ```ignore
/// let bytes: [u8; 36] = [...];
/// let authority = unpack_coption_pubkey(&bytes)?;
/// match authority.as_ref() {
///     Some(pubkey) => println!("Authority: {}", pubkey),
///     None => println!("No authority"),
/// }
/// ```
fn unpack_coption_pubkey(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    // Split the 36 bytes into tag (4 bytes) and body (32 bytes)
    let (tag, body) = array_refs![src, 4, 32];

    // Parse the tag as little-endian u32
    let tag_value = u32::from_le_bytes(*tag);

    // Return based on tag value
    match tag_value {
        // Tag 0 = None
        0 => Ok(COption::none()),

        // Tag 1 = Some(Pubkey)
        1 => {
            // Create Pubkey from the 32 body bytes
            let pubkey = Pubkey::new_from_array(*body);
            Ok(COption::some(pubkey))
        }

        // Any other tag is invalid
        _ => Err(TokenError::InvalidInstruction.into()),
    }
}

/// Pack a COption<Pubkey> into 36 bytes.
///
/// This is the inverse of `unpack_coption_pubkey`.
///
/// # Layout
///
/// Same as unpack:
/// - Bytes 0-3: Tag (0 for None, 1 for Some)
/// - Bytes 4-35: Pubkey (zeros if None)
///
/// # Arguments
///
/// * `src` - The COption to pack
/// * `dst` - Mutable reference to exactly 36 bytes
///
/// # Example
///
/// ```ignore
/// let authority = COption::some(my_pubkey);
/// let mut bytes = [0u8; 36];
/// pack_coption_pubkey(&authority, &mut bytes);
/// ```
fn pack_coption_pubkey(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    // Split destination into tag and body
    let (tag, body) = mut_array_refs![dst, 4, 32];

    match src.as_ref() {
        // Some(pubkey) - write tag 1 and the pubkey bytes
        Some(pubkey) => {
            *tag = 1u32.to_le_bytes();
            body.copy_from_slice(pubkey.as_ref());
        }

        // None - write tag 0 and zero out the body
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

    /// Test that packing and unpacking a Mint produces the same result.
    #[test]
    fn test_mint_pack_unpack_roundtrip() {
        // Create a test mint with all fields set
        let original = Mint {
            mint_authority: COption::some(Pubkey::new_unique()),
            supply: 1_000_000_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::some(Pubkey::new_unique()),
        };

        // Pack it
        let mut packed = [0u8; Mint::LEN];
        original.pack(&mut packed).unwrap();

        // Unpack it
        let unpacked = Mint::unpack(&packed).unwrap();

        // Should be identical
        assert_eq!(original, unpacked);
    }

    /// Test mint with no authorities (fixed supply, no freezing).
    #[test]
    fn test_mint_no_authorities() {
        let mint = Mint {
            mint_authority: COption::none(),
            supply: 21_000_000_000_000_000, // 21 million with 9 decimals
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::none(),
        };

        let mut packed = [0u8; Mint::LEN];
        mint.pack(&mut packed).unwrap();

        let unpacked = Mint::unpack(&packed).unwrap();

        assert!(unpacked.mint_authority.is_none());
        assert!(unpacked.freeze_authority.is_none());
        assert_eq!(unpacked.supply, 21_000_000_000_000_000);
    }

    /// Test that uninitialized mint (all zeros) has is_initialized = false.
    #[test]
    fn test_mint_uninitialized() {
        let packed = [0u8; Mint::LEN];
        let mint = Mint::unpack(&packed).unwrap();

        assert!(!mint.is_initialized);
        assert!(mint.mint_authority.is_none());
        assert_eq!(mint.supply, 0);
        assert_eq!(mint.decimals, 0);
        assert!(mint.freeze_authority.is_none());
    }

    /// Test the exact size.
    #[test]
    fn test_mint_size() {
        assert_eq!(Mint::LEN, 82);
        assert_eq!(std::mem::size_of::<[u8; Mint::LEN]>(), 82);
    }

    /// Test that wrong-sized input fails with unpack_from_slice.
    #[test]
    fn test_mint_wrong_size() {
        let too_small = [0u8; 81];
        assert!(Mint::unpack_from_slice(&too_small).is_err());

        let too_large = [0u8; 83];
        assert!(Mint::unpack_from_slice(&too_large).is_err());
    }

    /// Test invalid COption tag.
    #[test]
    fn test_mint_invalid_coption_tag() {
        let mut packed = [0u8; Mint::LEN];

        // Set an invalid tag (2) for mint_authority
        packed[0] = 2;
        packed[1] = 0;
        packed[2] = 0;
        packed[3] = 0;

        let result = Mint::unpack(&packed);
        assert!(result.is_err());
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHAT IS A MINT?
===============

Think of a Mint like the US Treasury for a currency:
- It defines what the currency is
- It tracks how much exists (supply)
- It controls who can "print" more (mint_authority)

Every token on Solana has exactly ONE Mint account.
- USDC has one Mint
- BONK has one Mint
- Your token will have one Mint

FIELD-BY-FIELD EXPLANATION
==========================

1. mint_authority: COption<Pubkey>
   
   Who can create new tokens.
   
   Example scenarios:
   - Your personal token: Your wallet is mint_authority
   - DeFi protocol: A PDA is mint_authority (program controls it)
   - Fixed supply: None (no one can ever mint more)
   
   Real examples:
   - USDC: Circle's admin key is mint_authority
   - BONK: Was minted all at once, then set to None

2. supply: u64
   
   Total tokens that exist right now.
   
   This is the "global" count. The sum of all token accounts.
   
   Example:
   - Mint supply: 1,000,000
   - Alice has: 300,000
   - Bob has: 200,000
   - Carol has: 500,000
   - Total: 1,000,000 ✓ (must match supply)

3. decimals: u8
   
   How to display the token.
   
   On-chain, everything is integers.
   Decimals tells UIs how to show it.
   
   Example with decimals = 6:
   - On-chain amount: 1500000
   - Display to user: "1.5 USDC"
   
   Example with decimals = 0:
   - On-chain amount: 1
   - Display to user: "1 NFT" (no decimals)

4. is_initialized: bool
   
   Safety flag.
   
   When you create an account, it's filled with zeros.
   is_initialized = false (byte is 0)
   
   After InitializeMint:
   is_initialized = true (byte is 1)
   
   Every instruction checks this first!

5. freeze_authority: COption<Pubkey>
   
   Who can freeze token accounts.
   
   Freezing = account can't transfer tokens.
   
   Used by:
   - Stablecoins (regulatory compliance)
   - Games (lock items during battles)
   - Security (stop stolen funds)
   
   Most DeFi tokens set this to None (can't freeze).

THE ARRAYREF CRATE
==================

We use arrayref for zero-cost byte manipulation.

array_ref![input, 0, 82]
- Creates a &[u8; 82] from input
- Panics if input.len() < 82
- No runtime overhead

array_refs![input, 36, 8, 1, 1, 36]
- Splits [u8; 82] into:
  - [u8; 36], [u8; 8], [u8; 1], [u8; 1], [u8; 36]
- Sizes must sum to total
- Compile-time checked

Why not just use slices?
- Fixed-size arrays enable optimizations
- Compiler knows exact sizes
- No bounds checks at runtime
- from_le_bytes() needs exact size

LITTLE-ENDIAN ENCODING
======================

u64::from_le_bytes([0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
= 1000

"Little-endian" means least significant byte first.

Number: 1000 = 0x3E8
Little-endian: [0xE8, 0x03, 0x00, ...]  (E8 is "little" end)
Big-endian:    [0x00, ..., 0x03, 0xE8] (E8 is "big" end)

Solana uses little-endian (matches x86/ARM processors).

COPTION VS OPTION
=================

Rust's Option<T> has "niche optimization":
- Option<&T> is same size as &T (null = None)
- Option<NonZeroU32> is same size as u32 (0 = None)

But Option<Pubkey> might be 32 or 33 bytes.
Layout can change between Rust versions.

Our COption<Pubkey> is ALWAYS 36 bytes:
- 4 bytes: tag (0 or 1)
- 32 bytes: pubkey

This gives us:
- Deterministic layout
- SPL Token compatibility
- No surprises

WHY 82 BYTES?
=============

36 (mint_authority)
+ 8 (supply)
+ 1 (decimals)
+ 1 (is_initialized)
+ 36 (freeze_authority)
= 82 bytes

This matches SPL Token exactly.
Existing tools expect this size.
Changing it would break compatibility.

SAFETY CONSIDERATIONS
=====================

1. Always check is_initialized
   - Uninitialized = garbage data
   - Could have any values

2. Check mint_authority before minting
   - If None, minting is impossible
   - Return MintAuthorityRequired error

3. Check freeze_authority before freezing
   - If None, freezing is impossible
   - Return FreezeAuthorityRequired error

4. Watch for overflow
   - supply + amount might overflow u64
   - Use checked_add() always

TESTING
=======

The tests demonstrate:
1. Roundtrip: pack → unpack → same data
2. Edge cases: all zeros, no authorities
3. Size validation: too small/large fails
4. Invalid data: bad tag values error

Always test your serialization!
One wrong byte offset = catastrophic bug.
*/