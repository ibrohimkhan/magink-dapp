#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(
    PSP34,
    Ownable,
    PSP34Mintable,
    PSP34Enumerable,
    PSP34Metadata
)]
#[openbrush::contract]
pub mod wizard {

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
    }

    #[overrider(PSP34Mintable)]
    #[openbrush::modifiers(only_owner)]
    fn mint(&mut self, account: AccountId, id: Id) -> Result<(), PSP34Error> {
        psp34::InternalImpl::_mint_to(self, account, id)
    }

    impl Wizard {
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut _instance = Self::default();

            ownable::Internal::_init_with_owner(&mut _instance, Self::env().caller());
            psp34::Internal::_mint_to(&mut _instance, Self::env().caller(), Id::U8(1))
                .expect("Can mint");

            let collection_id = psp34::PSP34Impl::collection_id(&_instance);

            metadata::Internal::_set_attribute(
                &mut _instance,
                collection_id.clone(),
                String::from("name"),
                String::from("Wizard34"),
            );

            metadata::Internal::_set_attribute(
                &mut _instance,
                collection_id,
                String::from("symbol"),
                String::from("WZ34"),
            );

            _instance
        }
    }
}