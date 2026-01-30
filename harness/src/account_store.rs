//! A trait for implementing an account store, to be used with the
/// `MolluskContext`.
use {
    solana_account::{Account, AccountSharedData},
    solana_pubkey::Pubkey,
    std::collections::HashMap,
};

/// A trait for implementing an account store, to be used with the
/// `MolluskContext`.
pub trait AccountStore {
    /// Returns the default account to be used when an account is not found.
    fn default_account(&self, _pubkey: &Pubkey) -> Account {
        Account::default()
    }

    /// Get an account at the given public key.
    fn get_account(&self, pubkey: &Pubkey) -> Option<AccountSharedData>;

    /// Store an account at the given public key.
    fn store_account(&mut self, pubkey: Pubkey, account: AccountSharedData);
}

impl AccountStore for HashMap<Pubkey, AccountSharedData> {
    fn get_account(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.get(pubkey).cloned()
    }

    fn store_account(&mut self, pubkey: Pubkey, account: AccountSharedData) {
        self.insert(pubkey, account);
    }
}
