#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::wizard::WizardRef;

#[openbrush::implementation(
    PSP34,
    Ownable,
    PSP34Mintable,
    PSP34Enumerable,
    PSP34Metadata
)]
#[openbrush::contract]
pub mod wizard {

    use ink::prelude::string::{
        String,
        ToString,
    };

    use ink::codegen::{
        EmitEvent,
        Env,
    };

    use openbrush::traits::Storage;

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct Wizard {
        #[storage_field]
        psp34: psp34::Data,

        #[storage_field]
        ownable: ownable::Data,

        #[storage_field]
        metadata: metadata::Data,

        #[storage_field]
        enumerable: enumerable::Data,

        max_supply: u64,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,

        #[ink(topic)]
        to: Option<AccountId>,

        #[ink(topic)]
        id: Id,
    }

    #[overrider(psp34::Internal)]
    fn _emit_transfer_event(
        &self,
        from: Option<AccountId>,
        to: Option<AccountId>,
        id: Id,
    ) {
        self.env().emit_event(Transfer { from, to, id });
    }

    #[overrider(PSP34Mintable)]
    #[openbrush::modifiers(only_owner)]
    fn mint(&mut self, account: AccountId, id: Id) -> Result<(), PSP34Error> {
        if PSP34Impl::total_supply(self) as u64 >= self.max_supply {
            return Err(PSP34Error::Custom(String::from("CollectionFull")))
        }

        psp34::InternalImpl::_mint_to(self, account, id)
    }

    impl Wizard {
        #[ink(constructor)]
        pub fn new(max_supply: u64) -> Self {
            let mut _instance = Self::default();

            ownable::Internal::_init_with_owner(&mut _instance, Self::env().caller());

            let collection_id = psp34::PSP34Impl::collection_id(&_instance);

            metadata::Internal::_set_attribute(
                &mut _instance,
                collection_id.clone(),
                String::from("name"),
                String::from("Wizard34"),
            );

            metadata::Internal::_set_attribute(
                &mut _instance,
                collection_id.clone(),
                String::from("symbol"),
                String::from("WZ34"),
            );

            metadata::Internal::_set_attribute(
                &mut _instance,
                collection_id,
                String::from("baseUri"),
                String::from("https://bafybeibwbgwzqigw7touxmixxvkd3wfcf2rcljgbt75na7rwwnw4ojgljy.ipfs.nftstorage.link/"),
            );

            _instance.max_supply = max_supply;

            _instance
        }

        #[ink(message)]
        pub fn max_supply(&self) -> u64 {
            self.max_supply
        }

        #[ink(message)]
        pub fn get_total_supply(&self) -> Balance {
            PSP34Impl::total_supply(self)
        }

        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn set_base_uri(&mut self, uri: String) -> Result<(), PSP34Error> {
            let id = PSP34Impl::collection_id(self);
            metadata::Internal::_set_attribute(self, id, String::from("baseUri"), uri);

            Ok(())
        }

        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn set_name(&mut self, name: String) -> Result<(), PSP34Error> {
            let id = PSP34Impl::collection_id(self);
            metadata::Internal::_set_attribute(self, id, String::from("name"), name);

            Ok(())
        }

        #[ink(message)]
        #[openbrush::modifiers(only_owner)]
        pub fn set_symbol(&mut self, symbol: String) -> Result<(), PSP34Error> {
            let id = PSP34Impl::collection_id(self);
            metadata::Internal::_set_attribute(self, id, String::from("symbol"), symbol);

            Ok(())
        }

        #[ink(message)]
        pub fn token_uri(&self, token_id: u64) -> Result<String, PSP34Error> {
            psp34::InternalImpl::_owner_of(
                self,
                &openbrush::contracts::psp34::extensions::metadata::Id::U64(token_id),
            )
            .ok_or(PSP34Error::TokenNotExists)?;

            let base_uri = PSP34MetadataImpl::get_attribute(
                self,
                PSP34Impl::collection_id(self),
                String::from("baseUri"),
            );

            let token_uri =
                base_uri.unwrap() + &token_id.to_string() + &String::from(".json");
            Ok(token_uri)
        }

        #[ink(message)]
        pub fn token_name(&self, token_id: u64) -> Result<String, PSP34Error> {
            psp34::InternalImpl::_owner_of(
                self,
                &openbrush::contracts::psp34::extensions::metadata::Id::U64(token_id),
            )
            .ok_or(PSP34Error::TokenNotExists)?;

            match PSP34MetadataImpl::get_attribute(
                self,
                PSP34Impl::collection_id(self),
                String::from("name"),
            ) {
                Some(value) => Ok(value),
                None => Ok(String::from("")),
            }
        }

        #[ink(message)]
        pub fn token_symbol(&self, token_id: u64) -> Result<String, PSP34Error> {
            psp34::InternalImpl::_owner_of(
                self,
                &openbrush::contracts::psp34::extensions::metadata::Id::U64(token_id),
            )
            .ok_or(PSP34Error::TokenNotExists)?;

            match PSP34MetadataImpl::get_attribute(
                self,
                PSP34Impl::collection_id(self),
                String::from("symbol"),
            ) {
                Some(value) => Ok(value),
                None => Ok(String::from("")),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use ink::{
            env::test,
            prelude::string::String,
        };

        use openbrush::contracts::psp34::*;

        const MAX_SUPPLY: u64 = 3;
        const BASE_URI: &str = "https://bafybeibwbgwzqigw7touxmixxvkd3wfcf2rcljgbt75na7rwwnw4ojgljy.ipfs.nftstorage.link/";

        fn default_accounts() -> test::DefaultAccounts<ink::env::DefaultEnvironment> {
            test::default_accounts::<Environment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<Environment>(sender);
        }

        fn init() -> Wizard {
            Wizard::new(MAX_SUPPLY)
        }

        #[ink::test]
        fn init_works() {
            let wizard = init();
            let collection_id = PSP34Impl::collection_id(&wizard);

            assert_eq!(
                metadata::PSP34MetadataImpl::get_attribute(
                    &wizard,
                    collection_id.clone(),
                    String::from("name")
                ),
                Some(String::from("Wizard34"))
            );

            assert_eq!(
                metadata::PSP34MetadataImpl::get_attribute(
                    &wizard,
                    collection_id.clone(),
                    String::from("symbol")
                ),
                Some(String::from("WZ34"))
            );

            assert_eq!(
                metadata::PSP34MetadataImpl::get_attribute(
                    &wizard,
                    collection_id.clone(),
                    String::from("baseUri")
                ),
                Some(String::from(BASE_URI))
            );

            assert_eq!(wizard.max_supply(), MAX_SUPPLY);
            assert_eq!(wizard.get_total_supply(), 0);
        }

        #[ink::test]
        fn mint_works() {
            let mut wizard = init();
            assert_eq!(wizard.get_total_supply(), 0);

            let accounts = default_accounts();
            assert_eq!(Ownable::owner(&wizard).unwrap(), accounts.alice);

            assert!(PSP34Mintable::mint(&mut wizard, accounts.bob, Id::U64(1)).is_ok());
            assert_eq!(wizard.get_total_supply(), 1);

            assert_eq!(PSP34Impl::owner_of(&wizard, Id::U64(1)), Some(accounts.bob));

            assert_eq!(
                PSP34EnumerableImpl::owners_token_by_index(&wizard, accounts.bob, 0),
                Ok(Id::U64(1))
            );

            assert_eq!(1, ink::env::test::recorded_events().count());
        }

        #[ink::test]
        fn mint_more_max_supply_fails() {
            let mut wizard = init();
            let accounts = default_accounts();

            assert!(PSP34Mintable::mint(&mut wizard, accounts.bob, Id::U64(1)).is_ok());
            assert!(PSP34Mintable::mint(&mut wizard, accounts.eve, Id::U64(2)).is_ok());
            assert!(
                PSP34Mintable::mint(&mut wizard, accounts.django, Id::U64(3)).is_ok()
            );

            assert_eq!(wizard.get_total_supply(), 3);

            assert!(PSP34Mintable::mint(&mut wizard, accounts.eve, Id::U64(4)).is_err());

            assert_eq!(
                PSP34Mintable::mint(&mut wizard, accounts.eve, Id::U64(4)),
                Err(PSP34Error::Custom(String::from("CollectionFull")))
            );

            assert_eq!(3, ink::env::test::recorded_events().count());
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut wizard = init();
            let accounts = default_accounts();

            assert_eq!(Ownable::owner(&wizard).unwrap(), accounts.alice);
            assert!(Ownable::transfer_ownership(&mut wizard, accounts.bob).is_ok());

            assert_ne!(Ownable::owner(&wizard).unwrap(), accounts.alice);
            assert_eq!(Ownable::owner(&wizard).unwrap(), accounts.bob);
        }

        #[ink::test]
        fn token_uri_works() {
            let mut wizard = init();
            let accounts = default_accounts();

            assert!(PSP34Mintable::mint(&mut wizard, accounts.bob, Id::U64(1)).is_ok());

            assert_eq!(wizard.token_uri(42), Err(PSP34Error::TokenNotExists));
            assert_eq!(
                wizard.token_uri(1),
                Ok(String::from(BASE_URI.to_owned() + "1.json"))
            );
        }

        #[ink::test]
        fn token_name_works() {
            let mut wizard = init();
            let accounts = default_accounts();

            assert!(PSP34Mintable::mint(&mut wizard, accounts.bob, Id::U64(1)).is_ok());

            assert_eq!(wizard.token_name(42), Err(PSP34Error::TokenNotExists));
            assert_eq!(wizard.token_name(1), Ok(String::from("Wizard34")));
        }

        #[ink::test]
        fn token_symbol_works() {
            let mut wizard = init();
            let accounts = default_accounts();

            assert!(PSP34Mintable::mint(&mut wizard, accounts.bob, Id::U64(1)).is_ok());

            assert_eq!(wizard.token_symbol(42), Err(PSP34Error::TokenNotExists));
            assert_eq!(wizard.token_symbol(1), Ok(String::from("WZ34")));
        }

        #[ink::test]
        fn set_base_uri_works() {
            const NEW_BASE_URI: &str = "new_uri/";

            let mut wizard = init();
            let accounts = default_accounts();

            assert!(PSP34Mintable::mint(&mut wizard, accounts.alice, Id::U64(1)).is_ok());
            let collection_id = PSP34Impl::collection_id(&wizard);

            assert!(wizard.set_base_uri(NEW_BASE_URI.into()).is_ok());
            assert_eq!(
                PSP34MetadataImpl::get_attribute(
                    &wizard,
                    collection_id,
                    String::from("baseUri")
                ),
                Some(String::from(NEW_BASE_URI))
            );

            set_sender(accounts.bob);
            assert_eq!(
                wizard.set_base_uri(NEW_BASE_URI.into()),
                Err(PSP34Error::Custom(String::from("O::CallerIsNotOwner")))
            );
        }

        #[ink::test]
        fn set_name_and_symbol_works() {
            const NEW_NAME: &str = "new_name";
            const NEW_SYMBOL: &str = "new_symbol";

            let mut wizard = init();
            let accounts = default_accounts();

            assert!(PSP34Mintable::mint(&mut wizard, accounts.alice, Id::U64(1)).is_ok());
            let collection_id = PSP34Impl::collection_id(&wizard);

            assert!(wizard.set_name(NEW_NAME.into()).is_ok());
            assert_eq!(
                PSP34MetadataImpl::get_attribute(
                    &wizard,
                    collection_id.clone(),
                    String::from("name")
                ),
                Some(String::from(NEW_NAME))
            );

            set_sender(accounts.bob);
            assert_eq!(
                wizard.set_name(NEW_NAME.into()),
                Err(PSP34Error::Custom(String::from("O::CallerIsNotOwner")))
            );

            set_sender(accounts.alice);
            assert!(wizard.set_symbol(NEW_SYMBOL.into()).is_ok());
            assert_eq!(
                PSP34MetadataImpl::get_attribute(
                    &wizard,
                    collection_id,
                    String::from("symbol")
                ),
                Some(String::from(NEW_SYMBOL))
            );

            set_sender(accounts.bob);
            assert_eq!(
                wizard.set_symbol(NEW_SYMBOL.into()),
                Err(PSP34Error::Custom(String::from("O::CallerIsNotOwner")))
            );
        }
    }
}
