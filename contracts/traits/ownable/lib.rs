#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::primitives::AccountId;

#[ink::trait_definition]
pub trait Ownable {
    /// Returns the owner of the contract.
    #[ink(message)]
    fn owner(&self) -> AccountId;

    /// Checks if the caller is the owner of the contract.
    #[ink(message)]
    fn is_owner(&self, account: AccountId) -> bool;

    /// Renounces the ownership of the contract.
    #[ink(message)]
    fn renounce_ownership(&mut self);

    /// Transfers the ownership of the contract to a new account.
    #[ink(message)]
    fn transfer_ownership(&mut self, new_owner: AccountId);
}
