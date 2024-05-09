#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fragments_round {
    use ckb_merkle_mountain_range::{Merge, MerkleProof, Result as MMRResult};
    use core::f32::consts::E;
    use core::marker::PhantomData;
    use fa_nft::FaNftRef;
    use ink::prelude::vec::Vec;
    use ink::{storage::traits::StorageLayout, ToAccountId};
    use sha3::Digest;

    #[ink(storage)]
    pub struct FragmentsRound {
        fragment_basics: Vec<FragmentBasic>,
        /// the FA Nft contract AccountId
        fa_nft: AccountId,
        mmr_root: Leaf,
    }

    #[ink::scale_derive(Decode, Encode, TypeInfo)]
    #[derive(Debug, Clone, PartialEq)]
    pub struct FragmentBasic {
        cid: u32,
        release_block: u32,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(
        StorageLayout, Eq, PartialEq, Clone, Debug, Default, serde::Serialize, serde::Deserialize,
    )]
    pub struct Leaf(pub Vec<u8>);
    impl From<Vec<u8>> for Leaf {
        fn from(data: Vec<u8>) -> Self {
            let mut hasher = sha3::Sha3_256::default();
            hasher.update(&data);
            let hash = hasher.finalize();
            Leaf(hash.to_vec().into())
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
            Ok(Leaf(hash.to_vec().into()))
        }
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Proof<T, M> {
        mmr_size: u64,
        proof: Vec<T>,
        merge: PhantomData<M>,
    }

    impl Into<MerkleProof<Leaf, MergeLeaves>> for Proof<Leaf, MergeLeaves> {
        fn into(self) -> MerkleProof<Leaf, MergeLeaves> {
            MerkleProof::<Leaf, MergeLeaves>::new(self.mmr_size, self.proof)
        }
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(PartialEq, Debug)]
    pub enum Error {
        FragmentNotAvailable,
        FragmentBasicNotFound,
        /// The fragment can't be proven. This does not mean that the fragment is invalid.
        /// But something went wrong during the proof verification.
        FragmentCantBeProven,
        FragmentProofInvalid,
    }

    struct FragmentDetail {
        lifespan: u32,
    }

    // TODO remove this default implementation once real implementation is done
    impl Default for FragmentDetail {
        fn default() -> Self {
            Self { lifespan: 10 }
        }
    }

    struct Fragment {
        basic: FragmentBasic,
        detail: Option<FragmentDetail>,
    }

    impl Fragment {
        fn is_available(&self) -> bool {
            self.detail.is_some()
        }
    }

    impl FragmentsRound {
        #[ink(constructor)]
        pub fn new(fragment_basics: Vec<FragmentBasic>, mmr_root: Leaf, fa_nft: AccountId) -> Self {
            // let store = MemStore::default();
            // let mut mmr = MemMMR::<_, MergeLeaves>::new(0, store);

            // let fa_nft = FaNftRef::new()
            //     .code_hash(fa_nft_code_hash)
            //     .endowment(0)
            //     .salt_bytes([0xde, 0xad, 0xbe, 0xef])
            //     .instantiate();

            Self {
                fragment_basics,
                fa_nft,
                mmr_root,
            }
        }

        /// Return all the Fragments in this round.
        #[ink(message)]
        pub fn get_fragments(&self) -> Vec<FragmentBasic> {
            self.fragment_basics.clone()
        }

        /// Check if the Fragment is available to be claimed by the caller.
        /// If it is available, it mints a Fragment Acknowledgement NFT.
        #[ink(message)]
        pub fn claim_fragment(
            &self,
            proof: Proof<Leaf, MergeLeaves>,
            pos: u64,
            otp: Vec<u8>,
        ) -> Result<(), Error> {
            let mmr_proof: MerkleProof<Leaf, MergeLeaves> = proof.into();
            let verifies = mmr_proof
                .verify(self.mmr_root.clone(), vec![(pos, Leaf::from(otp))])
                .map_err(|_| Error::FragmentCantBeProven)?;
            if !verifies {
                return Err(Error::FragmentProofInvalid);
            }
            // let mmr_proof: MerkleProof<Leaf, MergeLeaves> = mmr_proof.into();
            // let fragment = self.get_fragment(fragment_cid)?;

            // if fragment.is_available() {
            //     // Code to mint a Fragment Acknowledgement NFT
            //     Ok(())
            // } else {
            //     Err(Error::FragmentNotAvailable)
            // }
            Ok(())
        }

        /// Check if the caller is eligible to claim the reward.
        /// If it is, it calculates the reward and transfers it to the caller.
        #[ink(message)]
        pub fn get_reward(&self) {}

        /// Get the weight of the Fragment.
        /// The weight is used to calculate the reward.
        /// It's a number between 0 and 255.
        fn get_fragment_weight(&self, _fragment: &Fragment) -> Result<u8, Error> {
            todo!()
        }

        fn mint_fragment_acknowledgement(
            &self,
            _fragment: Fragment,
            _weight: u8,
        ) -> Result<(), Error> {
            todo!()
        }

        fn get_fragment(&self, fragment_cid: u32) -> Result<Fragment, Error> {
            Ok(Fragment {
                basic: self.get_fragment_basic(fragment_cid)?,
                detail: self.get_fragment_detail(fragment_cid)?,
            })
        }

        fn get_fragment_detail(&self, fragment_cid: u32) -> Result<Option<FragmentDetail>, Error> {
            if self.env().block_number() >= self.get_fragment_basic(fragment_cid)?.release_block {
                // Code to mint a Fragment Acknowledgement NFT
                Ok(Some(FragmentDetail::default()))
            } else {
                Ok(None)
            }
        }

        fn get_fragment_basic(&self, fragment_cid: u32) -> Result<FragmentBasic, Error> {
            if let Some(fragment_basic) = self
                .fragment_basics
                .iter()
                .find(|fragment| fragment.cid == fragment_cid)
            {
                Ok(fragment_basic.clone())
            } else {
                Err(Error::FragmentBasicNotFound)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::primitives::AccountId;
        use std::vec;

        fn mock_fragment_basic() -> FragmentBasic {
            FragmentBasic {
                cid: 1,
                release_block: 11,
            }
        }

        fn mock_round() -> FragmentsRound {
            let fragment_basics = [mock_fragment_basic()].to_vec();
            FragmentsRound {
                fragment_basics: fragment_basics,
                fa_nft: AccountId::from([0x01; 32]),
                mmr_root: Leaf::default(),
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
            let round = mock_round();
            assert_eq!(round.get_fragments(), [mock_fragment_basic()].to_vec());
        }

        #[ink::test]
        fn it_cant_claim_fragment_before_decrypting_block() {
            let round = mock_round();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(10);
            assert_eq!(
                round.claim_fragment(mock_proof(), 0, vec![0x01]),
                Err(Error::FragmentNotAvailable)
            );
        }

        #[ink::test]
        fn it_can_claim_fragment_after_decrypting_block() {
            let round = mock_round();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(11);
            assert_eq!(round.claim_fragment(mock_proof(), 0, vec![0x01]), Ok(()));
        }

        #[ink::test]
        fn it_cant_claim_fragmen_if_it_does_not_exist() {
            let round = mock_round();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(11);
            assert_eq!(
                round.claim_fragment(mock_proof(), 0, vec![0x01]),
                Err(Error::FragmentBasicNotFound)
            );
        }
    }
}
