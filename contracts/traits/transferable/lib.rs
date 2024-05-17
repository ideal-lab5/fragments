#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::primitives::AccountId;

type Balance = <ink::env::DefaultEnvironment as ink::env::Environment>::Balance;

#[ink::trait_definition]
pub trait Transferable {
    /// Transfers balance to the given account.
    #[ink(message)]
    fn transfer_balance(&mut self, to: AccountId, value: Balance);
}
