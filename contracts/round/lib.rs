#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fragments_round {
    use ckb_merkle_mountain_range::{Merge, MerkleProof, Result as MMRResult};
    use core::marker::PhantomData;
    use fa_nft::FaNftRef;
    use ink::prelude::{vec, vec::Vec};
    use ink::ToAccountId;
    use ownable::Ownable;
    use sha3::Digest;

    #[ink(storage)]
    pub struct FragmentsRound {
        fragments: Vec<Fragment>,
        /// the FA Nft contract AccountId
        fa_nft: AccountId,
        mmr_root: Vec<u8>,
        contract_owner: AccountId,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(PartialEq, Clone, Debug, Default)]
    pub struct Leaf(pub Vec<u8>);
    impl From<Vec<u8>> for Leaf {
        fn from(data: Vec<u8>) -> Self {
            let mut hasher = sha3::Sha3_256::default();
            hasher.update(&data);
            let hash = hasher.finalize();
            Leaf(hash.to_vec())
        }
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct MergeLeaves;

    impl Merge for MergeLeaves {
        type Item = Leaf;
        fn merge(lhs: &Self::Item, rhs: &Self::Item) -> MMRResult<Self::Item> {
            let mut hasher = sha3::Sha3_256::default();
            hasher.update(&lhs.0);
            hasher.update(&rhs.0);
            let hash = hasher.finalize();
            Ok(Leaf(hash.to_vec()))
        }
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Proof<T, M> {
        mmr_size: u64,
        proof: Vec<T>,
        merge: PhantomData<M>,
    }

    impl From<Proof<Leaf, MergeLeaves>> for MerkleProof<Leaf, MergeLeaves> {
        fn from(val: Proof<Leaf, MergeLeaves>) -> Self {
            MerkleProof::<Leaf, MergeLeaves>::new(val.mmr_size, val.proof)
        }
    }

    impl From<MerkleProof<Leaf, MergeLeaves>> for Proof<Leaf, MergeLeaves> {
        fn from(mmr_proof: MerkleProof<Leaf, MergeLeaves>) -> Self {
            Proof {
                mmr_size: mmr_proof.mmr_size(),
                proof: mmr_proof.proof_items().to_vec(),
                merge: PhantomData,
            }
        }
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(PartialEq, Debug)]
    pub enum Error {
        NotFound,
        /// The fragment can't be proven. This does not mean that the fragment is invalid.
        /// But something went wrong during the proof verification.
        CantBeProven,
        ProofInvalid,
        FaNFT(fa_nft::Error),
    }

    #[ink::scale_derive(Decode, Encode, TypeInfo)]
    #[derive(Debug, Clone, PartialEq)]
    pub struct Fragment {
        cid: u32,
        mmr_pos: u64,
        release_block: u32,
    }

    impl Ownable for FragmentsRound {
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

    // impl Transferable for FragmentsRound {
    //     #[ink(message)]
    //     fn transfer_balance(&mut self, to: AccountId, value: Balance) {
    //         self.ensure_owner();
    //         self.env().transfer(to, value).unwrap();
    //     }
    // }

    /// The `FragmentsRound` struct represents a round of fragments.
    /// It contains information about the fragment details, the MMR root, and the Fragment Acknowledge NFT account ID.
    impl FragmentsRound {
        /// Creates a new instance of `FragmentsRound`.
        ///
        /// # Arguments
        ///
        /// * `fragments` - A vector of `Fragment` instances, each representing the details of a fragment.
        /// * `mmr_root` - A byte vector representing the root of the Merkle Mountain Range (MMR), where each leaf corresponds to a fragment.
        /// * `fa_nft_code_hash` - The code hash of the deployed Fragment Acknowledge NFT (Non-Fungible Token).
        ///
        /// # Returns
        ///
        /// A new instance of `FragmentsRound`. This instance is linked to the Fragment Acknowledge NFT via its code hash, and is initialized with the provided fragments and MMR root.
        #[ink(constructor)]
        pub fn new(fragments: Vec<Fragment>, mmr_root: Vec<u8>, fa_nft_code_hash: Hash) -> Self {
            let fa_nft = FaNftRef::new()
                .code_hash(fa_nft_code_hash)
                .endowment(0)
                .salt_bytes([0xde, 0xad, 0xbe, 0xef])
                .instantiate()
                .to_account_id();

            Self {
                fragments,
                fa_nft,
                mmr_root,
                contract_owner: Self::env().caller(),
            }
        }

        /// Returns all the fragments in this round.
        ///
        /// # Returns
        ///
        /// A vector of `Fragment` representing all the fragments in this round.
        #[ink(message)]
        pub fn get_fragments(&self) -> Vec<Fragment> {
            self.fragments.clone()
        }

        /// Checks if the fragment is available to be claimed by the caller.
        /// If it is available, it mints a Fragment Acknowledgement NFT.
        ///
        /// # Arguments
        ///
        /// * `proof` - The proof of inclusion for the fragment in the MMR.
        /// * `cid` - The content identifier (CID) of the fragment
        /// * `hash` - The hash of the fragment.
        ///
        /// # Returns
        ///
        /// An `Ok` result if the fragment is successfully claimed and the NFT is minted, or an `Err` result with an `Error` if the claim fails.
        #[ink(message)]
        pub fn claim_fragment(
            &self,
            proof: Proof<Leaf, MergeLeaves>,
            cid: u32,
            hash: Vec<u8>,
        ) -> Result<(), Error> {
            let mmr_proof: MerkleProof<Leaf, MergeLeaves> = proof.into();
            let verifies = mmr_proof
                .verify(
                    Leaf(self.mmr_root.clone()),
                    vec![(self.get_fragment(cid)?.mmr_pos, Leaf::from(hash))],
                )
                .map_err(|_| Error::CantBeProven)?;
            if !verifies {
                return Err(Error::ProofInvalid);
            }

            self.mint_fragment_acknowledgement(cid)
        }

        /// Checks if the caller is eligible to claim the reward.
        /// If eligible, it calculates the reward and transfers it to the caller.
        #[ink(message)]
        pub fn claim_reward(&self) -> Result<(), Error> {
            // todo
            // calculate the reward and mint it to the caller
            Ok(())
        }

        /// Mints a fragment acknowledgement for the given position.
        ///
        /// # Arguments
        ///
        /// * `cid` - The content identifier (CID) of the fragment for which the acknowledgement is being minted.
        ///
        /// # Returns
        ///
        /// An `Ok` result if the fragment acknowledgement is successfully minted, or an `Err` result with an `Error` if minting fails.
        fn mint_fragment_acknowledgement(&self, cid: u32) -> Result<(), Error> {
            // todo
            // mint FA NFT to the caller with the current block number and the fragment CID
            let caller = Self::env().caller();
            let block_number: BlockNumber = Self::env().block_number();
            let mut fa_nft = self.get_fa_nft_contract();
            fa_nft
                .mint(cid, caller, block_number)
                .map_err(Error::FaNFT)?;
            Ok(())
        }

        fn get_fragment(&self, cid: u32) -> Result<&Fragment, Error> {
            self.fragments
                .iter()
                .find(|f| f.cid == cid)
                .ok_or(Error::NotFound)
        }

        fn get_fa_nft_contract(&self) -> FaNftRef {
            ink::env::call::FromAccountId::from_account_id(self.fa_nft)
        }

        /// Ensures that the caller is the contract owner.
        fn ensure_owner(&self) {
            assert!(
                self.is_owner(self.env().caller()),
                "Caller is not the contract owner"
            );
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ckb_merkle_mountain_range::util::{MemMMR, MemStore};
        use ink::{env::test::DefaultAccounts, primitives::AccountId};
        use std::vec;

        fn mock_fragment() -> Fragment {
            Fragment {
                cid: 1,
                mmr_pos: 10,
                release_block: 11,
            }
        }

        fn mock_round(
            mmr_root: Option<Vec<u8>>,
            fragments: Option<Vec<Fragment>>,
        ) -> FragmentsRound {
            FragmentsRound {
                fragments: fragments.unwrap_or([mock_fragment()].to_vec()),
                fa_nft: AccountId::from([0x01; 32]),
                mmr_root: mmr_root.unwrap_or(Vec::<u8>::default()),
                contract_owner: accounts().alice,
            }
        }

        fn accounts() -> DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn mock_proof() -> Proof<Leaf, MergeLeaves> {
            Proof {
                mmr_size: 1,
                proof: vec![Leaf::default()],
                merge: PhantomData,
            }
        }

        #[ink::test]
        fn it_returns_fragments() {
            let round = mock_round(None, None);
            assert_eq!(round.get_fragments(), [mock_fragment()].to_vec());
        }

        #[ink::test]
        fn it_cant_claim_fragment_with_invalid_proof() {
            let round = mock_round(None, Some(vec![mock_fragment()]));
            assert_eq!(
                round.claim_fragment(mock_proof(), 1, vec![0x01]),
                Err(Error::ProofInvalid)
            );
        }

        /// Test that we can claim a fragment by submitting a valid proof and leaf (pos + opt).
        /// First we replicate what would happen off-chain:
        /// 1. Create an MMR and its leafs.
        /// 2. Get its root
        /// 3. Create a new FragmentRound with the MMR root.
        /// 4. Generate a proof for a leaf.
        ///
        /// Finally we can assert that the fragment was claimed successfully.
        #[ink::test]
        fn it_can_claim_fragment_with_valid_proof() {
            /// Mock some HASH
            fn mock_hash_from_elem(elem: u32) -> Vec<u8> {
                sha3::Sha3_256::digest(&elem.to_be_bytes()).to_vec()
            }

            // Let's create a MMR with 8 leafs.
            let store = MemStore::default();
            let mut mmr = MemMMR::<_, MergeLeaves>::new(0, store);

            // 1. Create an MMR and its leafs.
            // We add 8 leafs to the MMR, each with a mocked HASH that uses the element as the seed.
            // The MMR has 14 nodes and looks like this:
            //           14
            //        /       \
            //      6          13
            //    /   \       /   \
            //   2     5     9     12
            //  / \   /  \  / \   /  \
            // 0   1 3   4 7   8 10  11
            //
            // `positions` maps the element to its leaf position in the MMR.
            // It looks like this:
            //  [
            //   0 => 0,
            //   1 => 1,
            //   2 => 3,
            //   3 => 4,
            //   4 => 7,
            //   5 => 8,
            //   6 => 10,
            //   7 => 11,
            //  ]
            let positions: Vec<u64> = (0u32..8)
                .map(|i| mmr.push(Leaf::from(mock_hash_from_elem(i))).unwrap())
                .collect();

            // 2. Get its root
            let root = mmr.get_root().expect("get root");

            // 3. Create a new FragmentRound with the MMR root.
            // The element that we know the proof for is 5.
            let proof_elem: u32 = 5;
            let fragment = Fragment {
                cid: 1,
                mmr_pos: positions[proof_elem as usize],
                release_block: 11,
            };
            let round = mock_round(Some(root.0), Some(vec![fragment.clone()]));

            // 4. Generate a proof for a leaf.
            let proof = mmr
                .gen_proof(vec![positions[proof_elem as usize]])
                .expect("gen proof");

            assert_eq!(
                round.claim_fragment(proof.into(), fragment.cid, mock_hash_from_elem(proof_elem)),
                Ok(())
            );
        }
    }
}
