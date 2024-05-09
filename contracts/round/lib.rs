#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fragments_round {
    use ckb_merkle_mountain_range::{Merge, MerkleProof, Result as MMRResult};
    use core::marker::PhantomData;
    use ink::prelude::{vec, vec::Vec};
    use sha3::Digest;

    #[ink(storage)]
    pub struct FragmentsRound {
        fragment_details: Vec<FragmentDetails>,
        /// the FA Nft contract AccountId
        fa_nft: AccountId,
        mmr_root: Vec<u8>,
    }

    #[ink::scale_derive(Decode, Encode, TypeInfo)]
    #[derive(Debug, Clone, PartialEq)]
    pub struct FragmentDetails {
        cid: u32,
        release_block: u32,
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
        DetailsNotFound,
        /// The fragment can't be proven. This does not mean that the fragment is invalid.
        /// But something went wrong during the proof verification.
        CantBeProven,
        ProofInvalid,
    }

    /// The `FragmentsRound` struct represents a round of fragments.
    /// It contains information about the fragment details, the MMR root, and the Fragment Acknowledge NFT account ID.
    impl FragmentsRound {
        /// Creates a new instance of `FragmentsRound`.
        ///
        /// # Arguments
        ///
        /// * `fragment_details` - A list of `FragmentDetails` representing the details of each fragment.
        /// * `mmr_root` - The root of the MMR (Merkle Mountain Range), where each leaf represents a fragment.
        /// * `fa_nft` - The account ID of the deployed Fragment Acknowledge NFT.
        ///
        /// # Returns
        ///
        /// A new instance of `FragmentsRound`.
        #[ink(constructor)]
        pub fn new(
            fragment_details: Vec<FragmentDetails>,
            mmr_root: Vec<u8>,
            fa_nft: AccountId,
        ) -> Self {
            Self {
                fragment_details,
                fa_nft,
                mmr_root,
            }
        }

        /// Returns all the fragments in this round.
        ///
        /// # Returns
        ///
        /// A vector of `FragmentDetails` representing all the fragments in this round.
        #[ink(message)]
        pub fn get_fragments(&self) -> Vec<FragmentDetails> {
            self.fragment_details.clone()
        }

        /// Checks if the fragment is available to be claimed by the caller.
        /// If it is available, it mints a Fragment Acknowledgement NFT.
        ///
        /// # Arguments
        ///
        /// * `proof` - The proof of inclusion for the fragment in the MMR.
        /// * `pos` - The node position of the leaf that represents the fragment in the MMR.
        /// * `hash` - The hash of the fragment.
        ///
        /// # Returns
        ///
        /// An `Ok` result if the fragment is successfully claimed and the NFT is minted, or an `Err` result with an `Error` if the claim fails.
        #[ink(message)]
        pub fn claim_fragment(
            &self,
            proof: Proof<Leaf, MergeLeaves>,
            pos: u64,
            hash: Vec<u8>,
        ) -> Result<(), Error> {
            let mmr_proof: MerkleProof<Leaf, MergeLeaves> = proof.into();
            let verifies = mmr_proof
                .verify(Leaf(self.mmr_root.clone()), vec![(pos, Leaf::from(hash))])
                .map_err(|_| Error::CantBeProven)?;
            if !verifies {
                return Err(Error::ProofInvalid);
            }

            self.mint_fragment_acknowledgement(pos)
        }

        /// Checks if the caller is eligible to claim the reward.
        /// If eligible, it calculates the reward and transfers it to the caller.
        #[ink(message)]
        pub fn claim_reward(&self) {}

        /// Mints a fragment acknowledgement for the given position.
        ///
        /// # Arguments
        ///
        /// * `_pos` - The position of the fragment in the MMR.
        ///
        /// # Returns
        ///
        /// An `Ok` result if the fragment acknowledgement is successfully minted, or an `Err` result with an `Error` if minting fails.
        fn mint_fragment_acknowledgement(&self, _pos: u64) -> Result<(), Error> {
            // todo
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ckb_merkle_mountain_range::util::{MemMMR, MemStore};
        use ink::primitives::AccountId;
        use std::vec;

        fn mock_fragment_basic() -> FragmentDetails {
            FragmentDetails {
                cid: 1,
                release_block: 11,
            }
        }

        fn mock_round(mmr_root: Option<Vec<u8>>) -> FragmentsRound {
            FragmentsRound {
                fragment_details: [mock_fragment_basic()].to_vec(),
                fa_nft: AccountId::from([0x01; 32]),
                mmr_root: mmr_root.unwrap_or(Vec::<u8>::default()),
            }
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
            let round = mock_round(None);
            assert_eq!(round.get_fragments(), [mock_fragment_basic()].to_vec());
        }

        #[ink::test]
        fn it_cant_claim_fragment_with_invalid_proof() {
            let round = mock_round(None);
            assert_eq!(
                round.claim_fragment(mock_proof(), 0, vec![0x01]),
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
            let round = mock_round(Some(root.0));

            // 4. Generate a proof for a leaf.
            // The element that we know the proof for is 5.
            let proof_elem: u32 = 5;
            let proof = mmr
                .gen_proof(vec![positions[proof_elem as usize]])
                .expect("gen proof");

            assert_eq!(
                round.claim_fragment(
                    proof.into(),
                    positions[proof_elem as usize],
                    mock_hash_from_elem(proof_elem)
                ),
                Ok(())
            );
        }
    }
}
