#![cfg_attr(not(feature = "std"), no_std, no_main)]
#[allow(clippy::new_without_default)]
#[ink::contract]
pub mod magink {
    use crate::ensure;

    use ink::storage::Mapping;

    use ink::prelude::string::String;

    use ink::env::{
        call::{
            build_call,
            ExecutionInput,
            Selector,
        },
        DefaultEnvironment,
    };

    use openbrush::contracts::psp34::{
        PSP34Error,
        *,
    };

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        TooEarlyToClaim,
        UserNotFound,
        MintFailed,
        NotAllBadgesCollected,
    }

    #[ink(storage)]
    pub struct Magink {
        user: Mapping<AccountId, Profile>,
        wizard_contract_account_id: AccountId,
        last_token_id: u64,
    }

    #[derive(
        Debug, PartialEq, Eq, PartialOrd, Ord, Clone, scale::Encode, scale::Decode,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Profile {
        // duration in blocks until next claim
        claim_era: u8,

        // block number of last claim
        start_block: u32,

        // number of badges claimed
        badges_claimed: u8,
    }

    impl Magink {
        /// Creates a new Magink smart contract.
        #[ink(constructor)]
        pub fn new(account_id: AccountId) -> Self {
            // this place would be greate to transfer ownership of wizard to magink, but there is no onchain account exist at this moment
            Self {
                user: Mapping::new(),
                wizard_contract_account_id: account_id,
                last_token_id: 1,
            }
        }

        /// Total supply of wizard tokens
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            build_call::<DefaultEnvironment>()
                .call(self.wizard_contract_account_id)
                .gas_limit(0)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!(
                    "get_total_supply"
                ))))
                .returns::<Balance>()
                .invoke()
        }

        /// (Re)Start the Magink the claiming era for the caller.
        #[ink(message)]
        pub fn start(&mut self, era: u8) {
            let profile = Profile {
                claim_era: era,
                start_block: self.env().block_number(),
                badges_claimed: 0,
            };

            self.user.insert(self.env().caller(), &profile);
        }

        /// Claim the badge after the era.
        #[ink(message)]
        pub fn claim(&mut self) -> Result<(), Error> {
            ensure!(self.get_remaining() == 0, Error::TooEarlyToClaim);

            // update profile
            let mut profile = self.get_profile().ok_or(Error::UserNotFound).unwrap();

            profile.badges_claimed += 1;
            profile.start_block = self.env().block_number();

            self.user.insert(self.env().caller(), &profile);

            Ok(())
        }

        /// Mint Wizard NFT
        #[ink(message)]
        pub fn mint_wizard(&mut self) -> Result<(), PSP34Error> {
            // assuming that exact number is configured in UI part
            ensure!(
                self.get_badges() > 0,
                PSP34Error::Custom(String::from("NotAllBadgesCollected"))
            );

            let caller = self.env().caller();
            build_call::<DefaultEnvironment>()
                .call(self.wizard_contract_account_id)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP34Mintable::mint"
                    )))
                    .push_arg(caller)
                    .push_arg(Id::U64(self.last_token_id)),
                )
                .returns::<Result<(), PSP34Error>>()
                .invoke()?;

            self.last_token_id += 1;
            Ok(())
        }

        /// Returns the remaining blocks in the era.
        #[ink(message)]
        pub fn get_remaining(&self) -> u8 {
            let current_block = self.env().block_number();
            let caller = self.env().caller();

            self.user.get(caller).map_or(0, |profile| {
                if current_block - profile.start_block >= profile.claim_era as u32 {
                    return 0
                }

                profile.claim_era - (current_block - profile.start_block) as u8
            })
        }

        /// Returns the remaining blocks in the era for the given account.
        #[ink(message)]
        pub fn get_remaining_for(&self, account: AccountId) -> u8 {
            let current_block = self.env().block_number();

            self.user.get(account).map_or(0, |profile| {
                if current_block - profile.start_block >= profile.claim_era as u32 {
                    return 0
                }

                profile.claim_era - (current_block - profile.start_block) as u8
            })
        }

        /// Returns the profile of the given account.
        #[ink(message)]
        pub fn get_account_profile(&self, account: AccountId) -> Option<Profile> {
            self.user.get(account)
        }

        /// Returns the profile of the caller.
        #[ink(message)]
        pub fn get_profile(&self) -> Option<Profile> {
            let caller = self.env().caller();
            self.user.get(caller)
        }

        /// Returns the badge of the caller.
        #[ink(message)]
        pub fn get_badges(&self) -> u8 {
            self.get_profile()
                .map_or(0, |profile| profile.badges_claimed)
        }

        /// Returns the badge count of the given account.
        #[ink(message)]
        pub fn get_badges_for(&self, account: AccountId) -> u8 {
            self.get_account_profile(account)
                .map_or(0, |profile| profile.badges_claimed)
        }
    }

    // cargo test --features e2e-tests -- --nocapture
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {

        use super::*;
        use crate::address_of;
        use wizard::WizardRef;

        use ink_e2e::{
            build_message,
            PolkadotConfig,
        };

        use openbrush::contracts::ownable::ownable_external::Ownable;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn check_default_owner_of_the_wizard_contract(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {
            let constructor = WizardRef::new(10);

            let account_id = client
                .instantiate("wizard", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let owner = {
                let msg = build_message::<WizardRef>(account_id.clone())
                    .call(|contract| contract.owner());
                client.call_dry_run(&ink_e2e::bob(), &msg, 0, None).await
            }
            .return_value();

            assert_eq!(owner, Some(address_of!(bob)));
            Ok(())
        }

        #[ink_e2e::test]
        async fn e2e_mint_works(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {
            let max_supply: u64 = 10;

            // instantiate wizard contract
            let wizard_constructor = WizardRef::new(max_supply);

            let wizard_account_id = client
                .instantiate("wizard", &ink_e2e::alice(), wizard_constructor, 0, None)
                .await
                .expect("wizard contract instantiate failed")
                .account_id;

            // instantiate magink contract
            let magink_constructor = MaginkRef::new(wizard_account_id);

            let magink_account_id = client
                .instantiate("magink", &ink_e2e::alice(), magink_constructor, 0, None)
                .await
                .expect("magink contract instantiate failed")
                .account_id;

            // transfer ownership to magink
            let change_owner = build_message::<WizardRef>(wizard_account_id.clone())
                .call(|p| p.transfer_ownership(magink_account_id));

            client
                .call(&ink_e2e::alice(), change_owner, 0, None)
                .await
                .expect("calling transfer_ownership failed");

            // verfy it
            let owner =
                build_message::<WizardRef>(wizard_account_id.clone()).call(|p| p.owner());

            let owner_result = client
                .call_dry_run(&ink_e2e::alice(), &owner, 0, None)
                .await
                .return_value()
                .unwrap();

            assert_eq!(owner_result, magink_account_id);

            // check total supply
            let total_supply = {
                let msg = build_message::<MaginkRef>(magink_account_id.clone())
                    .call(|contract| contract.total_supply());

                client.call_dry_run(&ink_e2e::alice(), &msg, 0, None).await
            }
            .return_value();

            assert_eq!(total_supply, 0);

            // start
            let start_msg = build_message::<MaginkRef>(magink_account_id.clone())
                .call(|magink| magink.start(0));

            client
                .call(&ink_e2e::alice(), start_msg, 0, None)
                .await
                .expect("calling start failed");

            // claim
            let claim_msg = build_message::<MaginkRef>(magink_account_id.clone())
                .call(|magink| magink.claim());

            client
                .call(&ink_e2e::alice(), claim_msg, 0, None)
                .await
                .expect("calling claim failed");

            let badges = {
                let msg = build_message::<MaginkRef>(magink_account_id.clone())
                    .call(|magink| magink.get_badges());

                client.call_dry_run(&ink_e2e::alice(), &msg, 0, None).await
            }
            .return_value();

            assert_eq!(badges, 1);

            // mint new token
            let mint_wizard_msg = build_message::<MaginkRef>(magink_account_id.clone())
                .call(|magink| magink.mint_wizard());

            client
                .call(&ink_e2e::alice(), mint_wizard_msg, 0, None)
                .await
                .expect("minting new token failed");

            let total_supply = {
                let msg = build_message::<MaginkRef>(magink_account_id.clone())
                    .call(|contract| contract.total_supply());

                client.call_dry_run(&ink_e2e::alice(), &msg, 0, None).await
            }
            .return_value();

            assert_eq!(total_supply, 1);

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn start_works() {
            let mut magink = Magink::new(AccountId::from([0x01; 32]));
            println!("get {:?}", magink.get_remaining());

            magink.start(10);
            assert_eq!(10, magink.get_remaining());

            advance_block();
            assert_eq!(9, magink.get_remaining());
        }

        #[ink::test]
        fn claim_works() {
            const ERA: u32 = 10;
            let accounts = default_accounts();

            let mut magink = Magink::new(AccountId::from([0x01; 32]));

            magink.start(ERA as u8);

            advance_n_blocks(ERA - 1);
            assert_eq!(1, magink.get_remaining());

            // claim fails, too early
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());

            // claim succeeds
            advance_block();
            assert_eq!(Ok(()), magink.claim());
            assert_eq!(1, magink.get_badges());
            assert_eq!(1, magink.get_badges_for(accounts.alice));
            assert_eq!(1, magink.get_badges());
            assert_eq!(10, magink.get_remaining());

            // claim fails, too early
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());

            advance_block();
            assert_eq!(9, magink.get_remaining());

            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());
        }

        #[ink::test]
        fn mint_check_works_offchain_contract_call_fails() {
            const ERA: u32 = 3;
            let mut magink = Magink::new(AccountId::from([0x01; 32]));

            magink.start(ERA as u8);

            assert_eq!(3, magink.get_remaining());
            assert_eq!(0, magink.get_badges());

            advance_block();
            assert_eq!(2, magink.get_remaining());
            advance_block();
            assert_eq!(1, magink.get_remaining());
            advance_block();
            assert_eq!(0, magink.get_remaining());

            // mint wizard returns error since not all badges are collected
            assert!(magink.mint_wizard().is_err());
            assert_eq!(
                magink.mint_wizard(),
                Err(PSP34Error::Custom(String::from("NotAllBadgesCollected")))
            );

            assert_eq!(Ok(()), magink.claim());
            assert_eq!(3, magink.get_remaining());
            assert_eq!(1, magink.get_badges());

            advance_block();
            assert_eq!(2, magink.get_remaining());
            advance_block();
            assert_eq!(1, magink.get_remaining());
            advance_block();
            assert_eq!(0, magink.get_remaining());

            assert_eq!(1, magink.get_badges());

            // panics due to off-chain environment does not support contract invocation
            let result = std::panic::catch_unwind(move || magink.mint_wizard());

            assert!(result.is_err());
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<Environment>()
        }

        fn advance_n_blocks(n: u32) {
            for _ in 0..n {
                advance_block();
            }
        }

        fn advance_block() {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }
}

/// Evaluate `$x:expr` and if not true return `Err($y:expr)`.
///
/// Used as `ensure!(expression_to_ensure, expression_to_return_on_false)`.
#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into())
        }
    }};
}

#[macro_export]
macro_rules! address_of {
    ($account:ident) => {
        ink::primitives::AccountId::from(
            ink_e2e::$account::<PolkadotConfig>().account_id().0,
        )
    };
}
