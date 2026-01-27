//! Multisig Account State
//!
//! A Multisig is an M-of-N authority.
//! It requires M signatures from N possible signers to authorize actions.
//!
//! # Real World Analogy
//!
//! Like a bank vault that needs 3 out of 5 keyholders to open:
//! - N = 5 (total keyholders)
//! - M = 3 (required to open)
//! - Any 3 of the 5 can open it together
//!
//! # Use Cases
//!
//! 1. Treasury Management
//!    - Company treasury needs 3/5 executives to approve spending
//!
//! 2. Protocol Governance
//!    - Upgrade authority is 4/7 multisig
//!
//! 3. Shared Custody
//!    - Family trust needs 2/3 members to access
//!
//! # Size: 355 bytes (matches SPL Token exactly)

use crate::error::TokenError;
use crate::state::Pack;
use arrayref::{array_mut_ref, array_ref};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of signers allowed in a multisig.
///
/// SPL Token allows up to 11 signers.
/// This is enough for most real-world scenarios.
///
/// Why 11?
/// - More signers = larger account = more rent
/// - More signers = more complex transactions
/// - 11 is sufficient for most governance needs
pub const MAX_SIGNERS: usize = 11;

// =============================================================================
// MULTISIG STRUCTURE
// =============================================================================

/// Multisig account data structure.
///
/// Represents an M-of-N multisig authority.
///
/// # Memory Layout (355 bytes total)
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │ Offset │ Size │ Field          │ Type                          │
/// ├────────┼──────┼────────────────┼───────────────────────────────┤
/// │ 0      │ 1    │ m              │ u8 (required signatures)      │
/// │ 1      │ 1    │ n              │ u8 (total signers)            │
/// │ 2      │ 1    │ is_initialized │ bool (as u8)                  │
/// │ 3      │ 352  │ signers        │ [Pubkey; 11] (32 * 11)        │
/// ├────────┼──────┼────────────────┼───────────────────────────────┤
/// │ Total  │ 355  │                │                               │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Example
///
/// Creating a 2-of-3 multisig:
/// ```ignore
/// let multisig = Multisig {
///     m: 2,  // Need 2 signatures
///     n: 3,  // Out of 3 possible signers
///     is_initialized: true,
///     signers: [alice, bob, carol, Pubkey::default(), ...], // Only first 3 matter
/// };
/// ```
///
/// # Using the Multisig
///
/// When used as mint_authority, freeze_authority, or owner:
/// 1. Pass the multisig account as the "authority"
/// 2. Pass M signer accounts after it
/// 3. Those signers must be in the multisig.signers list
/// 4. Those signers must have actually signed the transaction
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Multisig {
    /// Number of signatures required (M in M-of-N).
    ///
    /// # Constraints
    ///
    /// - Must be >= 1
    /// - Must be <= n
    ///
    /// # Examples
    ///
    /// - m=1: Any single signer can authorize (not very secure)
    /// - m=n: ALL signers must authorize (most secure, least flexible)
    /// - m=(n/2)+1: Majority must authorize (balanced)
    pub m: u8,

    /// Number of valid signers (N in M-of-N).
    ///
    /// # Constraints
    ///
    /// - Must be >= 1
    /// - Must be <= 11 (MAX_SIGNERS)
    /// - Must be >= m
    ///
    /// # Note
    ///
    /// Only the first `n` entries in `signers` array are valid.
    /// Remaining entries should be ignored.
    pub n: u8,

    /// Whether this multisig has been initialized.
    ///
    /// Same pattern as Mint and Account.
    pub is_initialized: bool,

    /// Array of signer public keys.
    ///
    /// # Layout
    ///
    /// - First `n` entries: Valid signer pubkeys
    /// - Remaining entries: Ignored (usually zeros)
    ///
    /// # Example
    ///
    /// For a 2-of-3 multisig:
    /// ```text
    /// signers[0] = Alice's pubkey  ← Valid
    /// signers[1] = Bob's pubkey    ← Valid
    /// signers[2] = Carol's pubkey  ← Valid
    /// signers[3] = zeros           ← Ignored
    /// ...
    /// signers[10] = zeros          ← Ignored
    /// ```
    ///
    /// # Uniqueness
    ///
    /// All valid signers should be unique.
    /// Duplicate signers would allow one person to count as multiple signatures.
    pub signers: [Pubkey; MAX_SIGNERS],
}

// =============================================================================
// DEFAULT IMPLEMENTATION
// =============================================================================

impl Default for Multisig {
    /// Create an empty, uninitialized multisig.
    fn default() -> Self {
        Self {
            m: 0,
            n: 0,
            is_initialized: false,
            signers: [Pubkey::default(); MAX_SIGNERS],
        }
    }
}

// =============================================================================
// ASSOCIATED CONSTANTS AND METHODS
// =============================================================================

impl Multisig {
    /// Size of Multisig when serialized.
    ///
    /// Calculation:
    /// - m: 1 byte
    /// - n: 1 byte
    /// - is_initialized: 1 byte
    /// - signers: 11 * 32 = 352 bytes
    /// - Total: 1 + 1 + 1 + 352 = 355 bytes
    pub const LEN: usize = 355;
}

// =============================================================================
// PACK TRAIT IMPLEMENTATION
// =============================================================================

impl Pack for Multisig {
    const LEN: usize = 355;

    /// Deserialize a Multisig from bytes.
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Create fixed-size reference
        let input = array_ref![input, 0, Multisig::LEN];

        // Parse fixed fields (first 3 bytes)
        let m = input[0];
        let n = input[1];
        let is_initialized = input[2] != 0;

        // =====================================================================
        // VALIDATION
        // =====================================================================

        // n must not exceed maximum
        if n as usize > MAX_SIGNERS {
            return Err(TokenError::InvalidMultisigConfig.into());
        }

        // m must not exceed n
        if m > n {
            return Err(TokenError::InvalidMultisigConfig.into());
        }

        // If initialized, m must be at least 1
        if is_initialized && m == 0 {
            return Err(TokenError::InvalidMultisigConfig.into());
        }

        // =====================================================================
        // PARSE SIGNERS
        // =====================================================================

        // Parse all 11 signer pubkeys
        // We always store 11, but only first `n` are valid
        let mut signers = [Pubkey::default(); MAX_SIGNERS];

        for i in 0..MAX_SIGNERS {
            // Calculate byte offset for this signer
            // Offset = 3 (header) + i * 32 (pubkey size)
            let start = 3 + i * 32;
            let end = start + 32;

            // Extract the 32 bytes
            let pubkey_bytes: [u8; 32] = input[start..end]
                .try_into()
                .map_err(|_| ProgramError::InvalidAccountData)?;

            // Convert to Pubkey
            signers[i] = Pubkey::new_from_array(pubkey_bytes);
        }

        Ok(Multisig {
            m,
            n,
            is_initialized,
            signers,
        })
    }

    /// Serialize a Multisig to bytes.
    fn pack(&self, output: &mut [u8]) -> Result<(), ProgramError> {
        // Create fixed-size mutable reference
        let output = array_mut_ref![output, 0, Multisig::LEN];

        // Write fixed fields (first 3 bytes)
        output[0] = self.m;
        output[1] = self.n;
        output[2] = self.is_initialized as u8;

        // Write all 11 signer pubkeys
        for i in 0..MAX_SIGNERS {
            let start = 3 + i * 32;
            let end = start + 32;
            output[start..end].copy_from_slice(self.signers[i].as_ref());
        }

        Ok(())
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
    fn test_multisig_pack_unpack_roundtrip() {
        let mut signers = [Pubkey::default(); MAX_SIGNERS];
        signers[0] = Pubkey::new_unique();
        signers[1] = Pubkey::new_unique();
        signers[2] = Pubkey::new_unique();

        let original = Multisig {
            m: 2,
            n: 3,
            is_initialized: true,
            signers,
        };

        let mut packed = [0u8; Multisig::LEN];
        original.pack(&mut packed).unwrap();

        let unpacked = Multisig::unpack(&packed).unwrap();

        assert_eq!(original, unpacked);
    }

    /// Test 1-of-1 multisig (edge case).
    #[test]
    fn test_multisig_one_of_one() {
        let mut signers = [Pubkey::default(); MAX_SIGNERS];
        signers[0] = Pubkey::new_unique();

        let multisig = Multisig {
            m: 1,
            n: 1,
            is_initialized: true,
            signers,
        };

        let mut packed = [0u8; Multisig::LEN];
        multisig.pack(&mut packed).unwrap();

        let unpacked = Multisig::unpack(&packed).unwrap();

        assert_eq!(multisig.m, unpacked.m);
        assert_eq!(multisig.n, unpacked.n);
    }

    /// Test maximum signers (11-of-11).
    #[test]
    fn test_multisig_max_signers() {
        let mut signers = [Pubkey::default(); MAX_SIGNERS];
        for i in 0..MAX_SIGNERS {
            signers[i] = Pubkey::new_unique();
        }

        let multisig = Multisig {
            m: 11,
            n: 11,
            is_initialized: true,
            signers,
        };

        let mut packed = [0u8; Multisig::LEN];
        multisig.pack(&mut packed).unwrap();

        let unpacked = Multisig::unpack(&packed).unwrap();

        assert_eq!(multisig.m, unpacked.m);
        assert_eq!(multisig.n, unpacked.n);
    }

    /// Test invalid: m > n.
    #[test]
    fn test_multisig_invalid_m_greater_than_n() {
        let mut packed = [0u8; Multisig::LEN];
        packed[0] = 3; // m = 3
        packed[1] = 2; // n = 2 (invalid: m > n)
        packed[2] = 1; // is_initialized = true

        let result = Multisig::unpack(&packed);
        assert!(result.is_err());
    }

    /// Test invalid: n > MAX_SIGNERS.
    #[test]
    fn test_multisig_invalid_n_too_large() {
        let mut packed = [0u8; Multisig::LEN];
        packed[0] = 1;  // m = 1
        packed[1] = 12; // n = 12 (invalid: > 11)
        packed[2] = 1;  // is_initialized = true

        let result = Multisig::unpack(&packed);
        assert!(result.is_err());
    }

    /// Test invalid: m = 0 when initialized.
    #[test]
    fn test_multisig_invalid_m_zero() {
        let mut packed = [0u8; Multisig::LEN];
        packed[0] = 0; // m = 0 (invalid when initialized)
        packed[1] = 3; // n = 3
        packed[2] = 1; // is_initialized = true

        let result = Multisig::unpack(&packed);
        assert!(result.is_err());
    }

    /// Test size is correct.
    #[test]
    fn test_multisig_size() {
        assert_eq!(Multisig::LEN, 355);
        assert_eq!(MAX_SIGNERS, 11);
        // 1 + 1 + 1 + (11 * 32) = 3 + 352 = 355
    }

    /// Test uninitialized multisig (all zeros is valid).
    #[test]
    fn test_multisig_uninitialized() {
        let packed = [0u8; Multisig::LEN];
        let multisig = Multisig::unpack(&packed).unwrap();

        assert_eq!(multisig.m, 0);
        assert_eq!(multisig.n, 0);
        assert!(!multisig.is_initialized);
    }
}

/*
=============================================================================
DETAILED EXPLANATION
=============================================================================

WHAT IS A MULTISIG?
===================

"Multisig" = "Multiple Signatures Required"

Instead of one person controlling something, 
you need multiple people to agree.

Real-world example:
- Corporate bank account needs 2 executives to sign checks
- Nuclear launch needs 2 officers with 2 keys
- Safe deposit box needs customer AND bank keys

SOLANA MULTISIG
===============

A Multisig account can be used as:
- mint_authority (control who mints tokens)
- freeze_authority (control who freezes accounts)
- owner of a token account
- Any other authority

When using a multisig authority:
1. Pass the multisig account info
2. Pass M signer account infos
3. Those M accounts must be in the multisig
4. Those M accounts must have signed

HOW IT'S VALIDATED
==================

In our validate_authority function (utils/authority.rs):

M-OF-N EXPLAINED
================

M = minimum signatures required
N = total possible signers

Examples:
- 1-of-1: Single signer (normal case)
- 2-of-3: Any 2 of 3 people
- 3-of-5: Any 3 of 5 people
- 5-of-5: ALL 5 must sign

Common patterns:
- 2-of-3: Small team, one backup
- 3-of-5: Medium team, good balance
- 4-of-7: Large team, majority rule

WHY 11 MAX?
===========

Practical limits:
- Each signer adds 32 bytes to account
- Each signer is an extra account in transaction
- Transactions have limited account count
- 11 covers most governance needs

If you need more:
- Use a governance program (Realms)
- Use recursive multisigs
- Use voting systems

SIZE CALCULATION
================

m:              1 byte
n:              1 byte
is_initialized: 1 byte
signers:        11 * 32 = 352 bytes
───────────────────────────
Total:          355 bytes

SECURITY CONSIDERATIONS
=======================

1. Signer Uniqueness
   - Same pubkey appearing twice = that person counts twice
   - Should validate during InitializeMultisig

2. M = 0
   - Would mean no signatures needed!
   - We reject this for initialized multisigs

3. Signer Order
   - Order doesn't matter during validation
   - We check if each signer is IN the list

4. Stale Signers
   - If a signer's key is compromised
   - Need to create new multisig with updated list
   - Cannot remove signers from existing multisig

TRANSACTION STRUCTURE
=====================

Example: Transfer using 2-of-3 multisig

Accounts:
0. source_token_account (writable)
1. dest_token_account (writable)
2. multisig_account (the 2-of-3 authority)
3. signer_1 (signer)
4. signer_2 (signer)

The program:
1. Sees account[2] is the authority
2. Detects it's a multisig (size 355, owned by us)
3. Loads multisig, sees m=2, n=3
4. Checks accounts[3] and [4]
5. Are they in multisig.signers? ✓
6. Are they is_signer? ✓
7. Count >= m? (2 >= 2) ✓
8. Authorized!
*/