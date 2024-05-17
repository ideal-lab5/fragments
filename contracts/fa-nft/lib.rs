//! # Fragment Acknowledgement NFT
//!
//! This is an ERC-721 Token implementation.
//!
//! ## Warning
//!
//! This contract is neither audited nor endorsed for production use.
//! Do **not** rely on it to keep anything of value secure.
//!
//! ## Error Handling
//!
//! Any function that modifies the state returns a `Result` type and does not changes the
//! state if the `Error` occurs.
//! The errors are defined as an `enum` type. Any other error or invariant violation
//! triggers a panic and therefore rolls back the transaction.
//!
//! ## Token Management
//!
//! After creating a new token, the function caller becomes the owner.
//! A token can be created, transferred, or destroyed.
//!
//! Token owners can assign other accounts for transferring specific tokens on their
//! behalf. It is also possible to authorize an operator (higher rights) for another
//! account to handle tokens.
//!
//! ### Token Creation
//!
//! Token creation start by calling the `mint(&mut self, id: u32)` function.
//! The token owner becomes the function caller. The Token ID needs to be specified
//! as the argument on this function call.
//!
//! ### Token Transfer
//!
//! Transfers may be initiated by:
//! - The owner of a token
//! - The approved address of a token
//! - An authorized operator of the current owner of a token
//!
//! The token owner can transfer a token by calling the `transfer` or `transfer_from`
//! functions. An approved address can make a token transfer by calling the
//! `transfer_from` function. Operators can transfer tokens on another account's behalf or
//! can approve a token transfer for a different account.
//!
//! ### Token Removal
//!
//! Tokens can be destroyed by burning them. Only the token owner is allowed to burn a
//! token.

#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::fa_nft::Error;
pub use self::fa_nft::FaNftRef;

#[ink::contract]
mod fa_nft {
    use ink::{
        env::hash::{Blake2x128, CryptoHash},
        scale::Encode,
        storage::Mapping,
    };
    use ownable::Ownable;
    use transferable::Transferable;

    /// A token ID.
    pub type TokenId = u64;

    pub type FragmentCid = u32;

    struct TokenRef(FragmentCid, AccountId, BlockNumber);

    impl From<TokenRef> for TokenId {
        fn from(input: TokenRef) -> Self {
            // TODO: use a cheaper hash method
            let mut output = [0u8; 16];
            Blake2x128::hash(
                &[
                    &input.0.encode()[..],
                    &input.1.encode()[..],
                    &input.2.encode()[..],
                ]
                .concat(),
                &mut output,
            );
            u64::from_be_bytes(output[0..8].try_into().unwrap())
        }
    }

    /// Information about a fragment acknowledgment.
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(Default, Debug, PartialEq)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub struct FragmentAcknowledgement {
        /// The fragment CID that was acknowledged.
        fragment_cid: FragmentCid,
        /// The block number when the fragment was acknowledged.
        block_number: BlockNumber,
    }

    #[derive(Debug, PartialEq, Eq, Copy, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        NotOwner,
        NotApproved,
        TokenExists,
        TokenNotFound,
        CannotInsert,
        CannotFetchValue,
        NotAllowed,
        NotContractOwner,
        TransferFailed,
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when a token approve occurs.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when an operator is enabled or disabled for an owner.
    /// The operator can manage all NFTs of the owner.
    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    #[ink(storage)]
    pub struct FaNft {
        /// Mapping from token to owner.
        token_owner: Mapping<TokenId, AccountId>,
        /// Mapping from token to approvals users.
        token_approvals: Mapping<TokenId, AccountId>,
        /// Mapping from owner to number of owned token.
        owned_tokens_count: Mapping<AccountId, u32>,
        /// Mapping from owner to operator approvals.
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
        /// The account ID of the contract owner.
        contract_owner: AccountId,
        /// Mapping from token to fragment acknowledgments.
        fragment_acknowledgments: Mapping<TokenId, FragmentAcknowledgement>,
    }

    impl Ownable for FaNft {
        #[ink(message)]
        fn owner(&self) -> AccountId {
            self.contract_owner
        }

        #[ink(message)]
        fn is_owner(&self, account: AccountId) -> bool {
            self.contract_owner == account
        }

        #[ink(message)]
        fn renounce_ownership(&mut self) {
            self.ensure_owner();
            self.contract_owner = AccountId::from([0x0; 32]);
        }

        #[ink(message)]
        fn transfer_ownership(&mut self, new_owner: AccountId) {
            self.ensure_owner();
            self.contract_owner = new_owner;
        }
    }

    impl Transferable for FaNft {
        #[ink(message)]
        fn transfer_balance(&mut self, to: AccountId, value: Balance) {
            self.ensure_owner();
            self.env().transfer(to, value).unwrap();
        }
    }

    impl FaNft {
        /// Creates a new ERC-721 token contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                token_owner: Default::default(),
                token_approvals: Default::default(),
                owned_tokens_count: Default::default(),
                operator_approvals: Default::default(),
                // deployer becomes owner
                contract_owner: Self::env().caller(),
                fragment_acknowledgments: Default::default(),
            }
        }

        /// Returns the balance of the owner.
        ///
        /// This represents the amount of unique tokens the owner has.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.balance_of_or_zero(&owner)
        }

        /// Returns the owner of the token.
        #[ink(message)]
        pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
            self.token_owner.get(id)
        }

        /// Returns the approved account ID for this token if any.
        #[ink(message)]
        pub fn get_approved(&self, id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(id)
        }

        /// Returns `true` if the operator is approved by the owner.
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.approved_for_all(owner, operator)
        }

        /// Approves or disapproves the operator for all tokens of the caller.
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
            self.approve_for_all(to, approved)?;
            Ok(())
        }

        /// Approves the account to transfer the specified token on behalf of the caller.
        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, id: TokenId) -> Result<(), Error> {
            self.approve_for(&to, id)?;
            Ok(())
        }

        /// Transfers the token from the caller to the given destination.
        #[ink(message)]
        pub fn transfer(&mut self, destination: AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.transfer_token_from(&caller, &destination, id)?;
            Ok(())
        }

        /// Transfer approved or owned token.
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            self.transfer_token_from(&from, &to, id)?;
            Ok(())
        }

        /// Creates a new token.
        #[ink(message)]
        pub fn mint(
            &mut self,
            fragment_cid: FragmentCid,
            owner: AccountId,
            block_number: BlockNumber,
        ) -> Result<TokenId, Error> {
            self.ensure_owner();

            let id = TokenRef(fragment_cid, owner, block_number).into();

            // store the fragment acknowledgment info
            self.fragment_acknowledgments.insert(
                id,
                &FragmentAcknowledgement {
                    fragment_cid,
                    block_number,
                },
            );

            self.add_token_to(&owner, id)?;
            self.env().emit_event(Transfer {
                from: Some(AccountId::from([0x0; 32])),
                to: Some(owner),
                id,
            });
            Ok(id)
        }

        /// Deletes an existing token. Only the owner can burn the token.
        #[ink(message)]
        pub fn burn(&mut self, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            let owner = token_owner.get(id).ok_or(Error::TokenNotFound)?;
            if owner != caller {
                return Err(Error::NotOwner);
            };

            let count = owned_tokens_count
                .get(caller)
                .map(|c| c.checked_sub(1).unwrap())
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(caller, &count);
            token_owner.remove(id);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: Some(AccountId::from([0x0; 32])),
                id,
            });

            Ok(())
        }

        /// Returns the fragment acknowledgment and token owner for the given token ID.
        #[ink(message)]
        pub fn get_fa_info(&self, id: TokenId) -> Option<(FragmentAcknowledgement, AccountId)> {
            if let (Some(fragment_acknowledgment), Some(token_owner)) = (
                self.fragment_acknowledgments.get(id),
                self.token_owner.get(id),
            ) {
                Some((fragment_acknowledgment, token_owner))
            } else {
                None
            }
        }

        /// Returns the fragment acknowledgment for the given token ID.
        #[ink(message)]
        pub fn get_fragment_acknowledgment(&self, id: TokenId) -> Option<FragmentAcknowledgement> {
            self.fragment_acknowledgments.get(id)
        }

        /// Transfers token `id` `from` the sender to the `to` `AccountId`.
        fn transfer_token_from(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;
            if !self.approved_or_owner(caller, id, owner) {
                return Err(Error::NotApproved);
            };
            if owner != *from {
                return Err(Error::NotOwner);
            };
            self.clear_approval(id);
            self.remove_token_from(from, id)?;
            self.add_token_to(to, id)?;
            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                id,
            });
            Ok(())
        }

        /// Removes token `id` from the owner.
        fn remove_token_from(&mut self, from: &AccountId, id: TokenId) -> Result<(), Error> {
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            if !token_owner.contains(id) {
                return Err(Error::TokenNotFound);
            }

            let count = owned_tokens_count
                .get(from)
                .map(|c| c.checked_sub(1).unwrap())
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(from, &count);
            token_owner.remove(id);

            Ok(())
        }

        /// Adds the token `id` to the `to` AccountID.
        fn add_token_to(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            let Self {
                token_owner,
                owned_tokens_count,
                ..
            } = self;

            if token_owner.contains(id) {
                return Err(Error::TokenExists);
            }

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            let count = owned_tokens_count
                .get(to)
                .map(|c| c.checked_add(1).unwrap())
                .unwrap_or(1);

            owned_tokens_count.insert(to, &count);
            token_owner.insert(id, to);

            Ok(())
        }

        /// Approves or disapproves the operator to transfer all tokens of the caller.
        fn approve_for_all(&mut self, to: AccountId, approved: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if to == caller {
                return Err(Error::NotAllowed);
            }
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator: to,
                approved,
            });

            if approved {
                self.operator_approvals.insert((&caller, &to), &());
            } else {
                self.operator_approvals.remove((&caller, &to));
            }

            Ok(())
        }

        /// Approve the passed `AccountId` to transfer the specified token on behalf of
        /// the message's sender.
        fn approve_for(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;
            if !(owner == caller || self.approved_for_all(owner, caller)) {
                return Err(Error::NotAllowed);
            };

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            if self.token_approvals.contains(id) {
                return Err(Error::CannotInsert);
            } else {
                self.token_approvals.insert(id, to);
            }

            self.env().emit_event(Approval {
                from: caller,
                to: *to,
                id,
            });

            Ok(())
        }

        /// Removes existing approval from token `id`.
        fn clear_approval(&mut self, id: TokenId) {
            self.token_approvals.remove(id);
        }

        // Returns the total number of tokens from an account.
        fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
            self.owned_tokens_count.get(of).unwrap_or(0)
        }

        /// Gets an operator on other Account's behalf.
        fn approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals.contains((&owner, &operator))
        }

        /// Returns true if the `AccountId` `from` is the owner of token `id`
        /// or it has been approved on behalf of the token `id` owner.
        fn approved_or_owner(&self, from: AccountId, id: TokenId, owner: AccountId) -> bool {
            from != AccountId::from([0x0; 32])
                && (from == owner
                    || self.token_approvals.get(id) == Some(from)
                    || self.approved_for_all(owner, from))
        }

        /// Ensures that the caller is the contract owner.
        fn ensure_owner(&self) {
            assert!(
                self.is_owner(self.env().caller()),
                "Caller is not the contract owner"
            );
        }
    }

    impl Default for FaNft {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        #[ink::test]
        fn mint_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            let token_id: TokenId =
                TokenRef(FragmentCid::default(), accounts.bob, BlockNumber::default()).into();
            // `token_id` does not exists.
            assert_eq!(fa_nft.owner_of(token_id), None);
            // Alice does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.bob), 0);
            // Create token.
            assert_eq!(
                fa_nft.mint(FragmentCid::default(), accounts.bob, BlockNumber::default()),
                Ok(token_id)
            );
            // Bob owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.bob), 1);
        }

        #[ink::test]
        fn mint_existing_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            let token_id: TokenId =
                TokenRef(FragmentCid::default(), accounts.bob, BlockNumber::default()).into();
            // Create token.
            assert_eq!(
                fa_nft.mint(FragmentCid::default(), accounts.bob, BlockNumber::default()),
                Ok(token_id)
            );
            // The first Transfer event takes place
            assert_eq!(1, ink::env::test::recorded_events().count());
            // Bob owns 1 token
            assert_eq!(fa_nft.balance_of(accounts.bob), 1);
            // Bob owns token
            assert_eq!(fa_nft.owner_of(token_id), Some(accounts.bob));
            // Cannot create token Id if it exists.
            // Token with same ID cannot be minted
            assert_eq!(
                fa_nft.mint(FragmentCid::default(), accounts.bob, BlockNumber::default()),
                Err(Error::TokenExists)
            );
        }

        #[ink::test]
        fn transfer_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Bob does not owns any token
            assert_eq!(fa_nft.balance_of(accounts.bob), 0);
            let token_id =
                fa_nft.mint(FragmentCid::default(), accounts.bob, BlockNumber::default());
            // Create token for Bob
            assert!(token_id.is_ok());
            // Bob owns 1 token
            assert_eq!(fa_nft.balance_of(accounts.bob), 1);
            // Charlie not owns any token
            assert_eq!(fa_nft.balance_of(accounts.charlie), 0);
            // The first Transfer event takes place
            assert_eq!(1, ink::env::test::recorded_events().count());
            // Change caller to Bob
            set_caller(accounts.bob);
            // Bob transfers token 1 to Charlie
            assert_eq!(fa_nft.transfer(accounts.charlie, token_id.unwrap()), Ok(()));
            // The second Transfer event takes place
            assert_eq!(2, ink::env::test::recorded_events().count());
            // Charlie owns 1 token
            assert_eq!(fa_nft.balance_of(accounts.charlie), 1);
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let token_id: TokenId = TokenRef(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            )
            .into();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Transfer token fails if it does not exists.
            assert_eq!(
                fa_nft.transfer(accounts.bob, token_id),
                Err(Error::TokenNotFound)
            );
            // Token Id does not exists.
            assert_eq!(fa_nft.owner_of(token_id), None);
            // Create token.
            assert_eq!(
                fa_nft.mint(
                    FragmentCid::default(),
                    accounts.alice,
                    BlockNumber::default()
                ),
                Ok(token_id)
            );
            // Alice owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.alice), 1);
            // Token is owned by Alice.
            assert_eq!(fa_nft.owner_of(token_id), Some(accounts.alice));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob cannot transfer not owned tokens.
            assert_eq!(
                fa_nft.transfer(accounts.eve, token_id),
                Err(Error::NotApproved)
            );
        }

        #[ink::test]
        fn approved_transfer_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token.
            let token_id = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_id.is_ok());
            let token_id = token_id.unwrap();
            // Token is owned by Alice.
            assert_eq!(fa_nft.owner_of(token_id), Some(accounts.alice));
            // Approve token transfer for Bob on behalf of Alice.
            assert_eq!(fa_nft.approve(accounts.bob, token_id), Ok(()));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob transfers token Id 1 from Alice to Eve.
            assert_eq!(
                fa_nft.transfer_from(accounts.alice, accounts.eve, token_id),
                Ok(())
            );
            // TokenId 3 is owned by Eve.
            assert_eq!(fa_nft.owner_of(token_id), Some(accounts.eve));
            // Alice does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.alice), 0);
            // Bob does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.bob), 0);
            // Eve owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.eve), 1);
        }

        #[ink::test]
        fn approved_for_all_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token 1.
            let token_1 = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_1.is_ok());
            let token_1 = token_1.unwrap();
            // Create token 2.
            let token_2 = fa_nft.mint(
                FragmentCid::default() + 1,
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_2.is_ok());
            let token_2 = token_2.unwrap();
            // Alice owns 2 tokens.
            assert_eq!(fa_nft.balance_of(accounts.alice), 2);
            // Approve token Id 1 transfer for Bob on behalf of Alice.
            assert_eq!(fa_nft.set_approval_for_all(accounts.bob, true), Ok(()));
            // Bob is an approved operator for Alice
            assert!(fa_nft.is_approved_for_all(accounts.alice, accounts.bob));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob transfers token 1 from Alice to Eve.
            assert_eq!(
                fa_nft.transfer_from(accounts.alice, accounts.eve, token_1),
                Ok(())
            );
            // Token 1 is owned by Eve.
            assert_eq!(fa_nft.owner_of(token_1), Some(accounts.eve));
            // Alice owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.alice), 1);
            // Bob transfers token 2 from Alice to Eve.
            assert_eq!(
                fa_nft.transfer_from(accounts.alice, accounts.eve, token_2),
                Ok(())
            );
            // Bob does not own tokens.
            assert_eq!(fa_nft.balance_of(accounts.bob), 0);
            // Eve owns 2 tokens.
            assert_eq!(fa_nft.balance_of(accounts.eve), 2);
            // Remove operator approval for Bob on behalf of Alice.
            set_caller(accounts.alice);
            assert_eq!(fa_nft.set_approval_for_all(accounts.bob, false), Ok(()));
            // Bob is not an approved operator for Alice.
            assert!(!fa_nft.is_approved_for_all(accounts.alice, accounts.bob));
        }

        #[ink::test]
        fn approve_nonexistent_token_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Approve transfer of nonexistent token id 1
            assert_eq!(fa_nft.approve(accounts.bob, 1), Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn not_approved_transfer_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token.
            let token_id = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_id.is_ok());
            // Alice owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.alice), 1);
            // Bob does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.bob), 0);
            // Eve does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.eve), 0);
            // Set Eve as caller
            set_caller(accounts.eve);
            // Eve is not an approved operator by Alice.
            assert_eq!(
                fa_nft.transfer_from(accounts.alice, accounts.frank, token_id.unwrap()),
                Err(Error::NotApproved)
            );
            // Alice owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.alice), 1);
            // Bob does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.bob), 0);
            // Eve does not owns tokens.
            assert_eq!(fa_nft.balance_of(accounts.eve), 0);
        }

        #[ink::test]
        fn burn_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token for Alice
            let token_id = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_id.is_ok());
            let token_id = token_id.unwrap();
            // Alice owns 1 token.
            assert_eq!(fa_nft.balance_of(accounts.alice), 1);
            // Alice owns token.
            assert_eq!(fa_nft.owner_of(token_id), Some(accounts.alice));
            // Destroy token.
            assert_eq!(fa_nft.burn(token_id), Ok(()));
            // Alice does not own tokens.
            assert_eq!(fa_nft.balance_of(accounts.alice), 0);
            // Token does not exists
            assert_eq!(fa_nft.owner_of(token_id), None);
        }

        #[ink::test]
        fn burn_fails_token_not_found() {
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Try burning a non existent token
            assert_eq!(fa_nft.burn(1), Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn burn_fails_not_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token for Alice
            let token_id = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_id.is_ok());
            // Try burning this token with a different account
            set_caller(accounts.eve);
            assert_eq!(fa_nft.burn(token_id.unwrap()), Err(Error::NotOwner));
        }

        #[ink::test]
        fn transfer_from_fails_not_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token for Alice
            let token_1 = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_1.is_ok());
            // Bob can transfer alice's tokens
            assert_eq!(fa_nft.set_approval_for_all(accounts.bob, true), Ok(()));
            // Create token for Frank
            assert!(fa_nft
                .mint(
                    FragmentCid::default(),
                    accounts.frank,
                    BlockNumber::default()
                )
                .is_ok());
            // Set caller to Bob
            set_caller(accounts.bob);
            // Bob makes invalid call to transfer_from (Alice is token owner, not Frank)
            assert_eq!(
                fa_nft.transfer_from(accounts.frank, accounts.bob, token_1.unwrap()),
                Err(Error::NotOwner)
            );
        }

        #[ink::test]
        fn transfer_fails_not_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Create token for Alice
            let token_id = fa_nft.mint(
                FragmentCid::default(),
                accounts.alice,
                BlockNumber::default(),
            );
            assert!(token_id.is_ok());
            // Bob can transfer alice's tokens
            assert_eq!(fa_nft.set_approval_for_all(accounts.bob, true), Ok(()));
            // Set caller to bob
            set_caller(accounts.bob);
            // Bob makes invalid call to transfer (he is not token owner, Alice is
            // and should use `transfer_from` instead)
            assert_eq!(
                fa_nft.transfer(accounts.bob, token_id.unwrap()),
                Err(Error::NotOwner)
            );
        }

        #[ink::test]
        fn ownership_can_be_transferred() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Alice is the contract owner
            assert_eq!(fa_nft.contract_owner, accounts.alice);
            fa_nft.transfer_ownership(accounts.bob);
            // Bob is now contract owner
            assert_eq!(fa_nft.contract_owner, accounts.bob);
        }

        #[ink::test]
        #[should_panic(expected = "Caller is not the contract owner")]
        fn ownership_cant_be_transferred_if_not_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut fa_nft = FaNft::default();
            // Set caller to Bob
            set_caller(accounts.bob);
            fa_nft.transfer_ownership(accounts.bob)
        }

        #[ink::test]
        fn transfer_balance_works() {
            let mut fa_nft = FaNft::default();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Alice is owner
            // Transfer some balance
            fa_nft.transfer_balance(accounts.bob, 50);
        }

        #[ink::test]
        #[should_panic(expected = "Caller is not the contract owner")]
        fn transfer_balance_fails_if_not_contract_owner() {
            let mut fa_nft = FaNft::default();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Set the contract owner to a different account
            fa_nft.contract_owner = accounts.eve;

            // Attempt to transfer balance should fail
            let _ = fa_nft.transfer_balance(accounts.bob, 50);
        }

        #[ink::test]
        fn get_fa_info_works() {
            let mut fa_nft = FaNft::default();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Initialize some data
            let token_id = 1;
            let fragment_acknowledgment = FragmentAcknowledgement::default();
            let owner = accounts.alice;

            // Set the data in the contract
            fa_nft
                .fragment_acknowledgments
                .insert(token_id, &fragment_acknowledgment);
            fa_nft.token_owner.insert(token_id, &owner);

            // Call get_fa_info and check the result
            let result = fa_nft.get_fa_info(token_id);
            assert_eq!(result, Some((fragment_acknowledgment, owner)));
        }

        #[ink::test]
        fn get_fa_info_returns_none_if_not_found() {
            let fa_nft = FaNft::new();

            // Call get_fa_info with a non-existent token ID
            let result = fa_nft.get_fa_info(1);
            assert_eq!(result, None);
        }

        #[ink::test]
        fn get_fragment_acknowledgment_works() {
            let mut fa_nft = FaNft::default();

            // Initialize some data
            let token_id = 1;
            let fragment_acknowledgment = FragmentAcknowledgement::default();
            // Set the data in the contract
            fa_nft
                .fragment_acknowledgments
                .insert(token_id, &fragment_acknowledgment);

            // Call get_fragment_acknowledgment and check the result
            let result = fa_nft.get_fragment_acknowledgment(token_id);
            assert_eq!(result, Some(fragment_acknowledgment));
        }

        #[ink::test]
        fn get_fragment_acknowledgment_returns_none_if_not_found() {
            let fa_nft = FaNft::new();

            // Call get_fragment_acknowledgment with a non-existent token ID
            let result = fa_nft.get_fragment_acknowledgment(1);
            assert_eq!(result, None);
        }

        #[ink::test]
        fn owner_returns_correct_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let fa_nft = FaNft::default();
            assert_eq!(fa_nft.owner(), accounts.alice);
        }

        #[ink::test]
        fn is_owner_returns_true_for_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let fa_nft = FaNft::default();
            assert!(fa_nft.is_owner(accounts.alice));
        }

        #[ink::test]
        fn is_owner_returns_false_for_non_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let fa_nft = FaNft::default();
            assert!(!fa_nft.is_owner(accounts.bob));
        }

        #[ink::test]
        fn renounce_ownership_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut fa_nft = FaNft::default();
            fa_nft.renounce_ownership();
            assert!(!fa_nft.is_owner(accounts.alice));
        }

        fn set_caller(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }
    }
}
