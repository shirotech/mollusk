//! Instruction <-> Transaction key deduplication and privilege handling.
//!
//! Solana instructions and transactions are designed to be intentionally
//! verbosely declarative, to provide the runtime with granular directives
//! for manipulating chain state.
//!
//! As a result, when a transaction is _compiled_, many steps occur:
//! * Ensuring there is a fee payer.
//! * Ensuring there is a signature.
//! * Deduplicating account keys.
//! * Configuring the highest role awarded to each account key.
//! * ...
//!
//! This modules provides utilities for deduplicating account keys and
//! handling the highest role awarded to each account key. It can be used
//! standalone or within the other transaction helpers in this library to build
//! custom transactions for the SVM API with the required structure and roles.
//!
//! This implementation closely follows the implementation in the Anza SDK
//! for `Message::new_with_blockhash`. For more information, see:
//! <https://github.com/anza-xyz/agave/blob/c6e8239843af8e6301cd198e39d0a44add427bef/sdk/program/src/message/legacy.rs#L357>.

use {
    ahash::HashSet,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    std::collections::BTreeMap,
};

/// Wrapper around a btree map of account keys and their corresponding roles
/// (`is_signer`, `is_writable`).
///
/// On compilation, keys are awarded the highest role they are assigned in the
/// transaction, and the btree map provides deduplication and deterministic
/// ordering.
///
/// The map can be queried by key for `is_signer` and `is_writable` roles.
#[derive(Debug, Default)]
pub struct KeyMap {
    map: BTreeMap<Pubkey, (bool, bool)>,
    program_ids: HashSet<Pubkey>,
}

impl KeyMap {
    /// Add a single account meta to the key map.
    pub fn add_account(&mut self, meta: &AccountMeta) {
        let entry = self.map.entry(meta.pubkey).or_default();
        entry.0 |= meta.is_signer;
        entry.1 |= meta.is_writable;
    }

    /// Add a list of account metas to the key map.
    pub fn add_accounts<'a>(&mut self, accounts: impl Iterator<Item = &'a AccountMeta>) {
        for meta in accounts {
            self.add_account(meta);
        }
    }

    /// Add keys from a single instruction to the key map.
    pub fn add_instruction(&mut self, instruction: &Instruction) {
        self.add_program(instruction.program_id);
        self.add_accounts(instruction.accounts.iter());
    }

    /// Add a single program ID to the key map.
    pub fn add_program(&mut self, program_id: Pubkey) {
        self.map.insert(program_id, (false, false));
        self.program_ids.insert(program_id);
    }

    pub fn compile_from_instruction(instruction: &Instruction) -> Self {
        let mut map = Self::default();
        map.add_instruction(instruction);
        map
    }

    /// Query the key map for the `is_signer` role of a key at the specified
    /// index.
    pub fn is_signer_at_index(&self, index: usize) -> bool {
        self.map
            .values()
            .nth(index)
            .map(|(s, _)| *s)
            .unwrap_or(false)
    }

    /// Query the key map for the `is_writable` role of a key at the specified
    /// index.
    pub fn is_writable_at_index(&self, index: usize) -> bool {
        self.map
            .values()
            .nth(index)
            .map(|(_, w)| *w)
            .unwrap_or(false)
    }

    /// Get the keys in the key map.
    pub fn keys(&self) -> impl Iterator<Item = &Pubkey> {
        self.map.keys()
    }

    /// Get the position of a key in the key map.
    ///
    /// This returns its position in the hash map's keys iterator.
    pub fn position(&self, key: &Pubkey) -> Option<usize> {
        self.map.keys().position(|k| k == key)
    }
}
