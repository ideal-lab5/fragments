#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fragments_round {
    use ink::prelude::vec::Vec;

    #[ink(storage)]
    pub struct FragmentsRound {
        fragment_basics: Vec<FragmentBasic>,
    }

    #[ink::scale_derive(Decode, Encode, TypeInfo)]
    #[derive(Debug, Clone, PartialEq)]
    pub struct FragmentBasic {
        cid: u32,
        release_block: u32,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(PartialEq, Debug)]
    pub enum Error {
        FragmentNotAvailable,
        FragmentBasicNotFound,
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
        pub fn new(fragment_basics: Vec<FragmentBasic>) -> Self {
            Self {
                fragment_basics: fragment_basics,
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
        pub fn claim_fragment(&self, fragment_cid: u32) -> Result<(), Error> {
            let fragment = self.get_fragment(fragment_cid)?;

            if fragment.is_available() {
                // Code to mint a Fragment Acknowledgement NFT
                Ok(())
            } else {
                Err(Error::FragmentNotAvailable)
            }
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

        fn mock_fragment_basic() -> FragmentBasic {
            FragmentBasic {
                cid: 1,
                release_block: 11,
            }
        }

        fn mock_round() -> FragmentsRound {
            let fragment_basics = [mock_fragment_basic()].to_vec();
            FragmentsRound::new(fragment_basics)
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
            assert_eq!(round.claim_fragment(1), Err(Error::FragmentNotAvailable));
        }

        #[ink::test]
        fn it_can_claim_fragment_after_decrypting_block() {
            let round = mock_round();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(11);
            assert_eq!(round.claim_fragment(1), Ok(()));
        }

        #[ink::test]
        fn it_cant_claim_fragmen_if_it_does_not_exist() {
            let round = mock_round();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(11);
            assert_eq!(round.claim_fragment(2), Err(Error::FragmentBasicNotFound));
        }
    }
}
