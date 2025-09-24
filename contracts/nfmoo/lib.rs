#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod nfmoo {
    use ink::storage::Mapping;
    use ink::prelude::vec::Vec;
    use core::cmp::min;

    pub type TokenId = u128;
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(scale::Encode, scale::Decode, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        AmountZero,
        Overflow,
        SameAccount,
        NotOwner,
        TokenMissing,
    }

    #[ink(event)]
    pub struct NFMinted {
        #[ink(topic)]
        to_acc: AccountId,
        #[ink(topic)]
        token_id: TokenId,
    }

    #[ink(event)]
    pub struct NFTransferred {
        #[ink(topic)]
        from_acc: AccountId,
        #[ink(topic)]
        to_acc: AccountId,
        #[ink(topic)]
        token_id: TokenId,
    }

    #[ink(event)]
    pub struct NFBurned {
        #[ink(topic)]
        from_acc: AccountId,
        #[ink(topic)]
        token_id: TokenId,
    }

    #[ink(storage)]
    pub struct NFMoo {
        /// Next token id to assign on mint.
        next_id: TokenId,

        /// token_id -> owner_acc
        owner_of: Mapping<TokenId, AccountId>,

        /// owner_acc -> number of tokens owned
        owned_count: Mapping<AccountId, u32>,

        /// (owner_acc, index) -> token_id (for pagination/enumeration)
        tokens_by_owner: Mapping<(AccountId, u32), TokenId>,

        /// token_id -> index within owner's enumeration list
        owned_index: Mapping<TokenId, u32>,
    }

    impl NFMoo {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                next_id: 0,
                owner_of: Mapping::default(),
                owned_count: Mapping::default(),
                tokens_by_owner: Mapping::default(),
                owned_index: Mapping::default(),
            }
        }

        // --------- mint / burn / transfer ---------

        /// Open mint: anyone can mint `amount` unique tokens to themselves.
        #[ink(message)]
        pub fn mint_n(&mut self, amount: u32) -> Result<()> {
            if amount == 0 {
                return Err(Error::AmountZero)
            }
            let to_acc = self.env().caller();
            for _ in 0..amount {
                let token_id = self.next_id;
                self.next_id = self.next_id.checked_add(1).ok_or(Error::Overflow)?;

                self.owner_of.insert(&token_id, &to_acc);
                self.add_token_to_owner(to_acc, token_id)?;
                self.env().emit_event(NFMinted { to_acc, token_id });
            }
            Ok(())
        }

        /// Transfer a token you own to `to_acc`.
        #[ink(message)]
        pub fn transfer(&mut self, to_acc: AccountId, token_id: TokenId) -> Result<()> {
            let from_acc = self
                .owner_of
                .get(&token_id)
                .ok_or(Error::TokenMissing)?;

            if from_acc != self.env().caller() {
                return Err(Error::NotOwner)
            }
            if from_acc == to_acc {
                return Err(Error::SameAccount)
            }

            self.remove_token_from_owner(from_acc, token_id)?;
            self.owner_of.insert(&token_id, &to_acc);
            self.add_token_to_owner(to_acc, token_id)?;

            self.env().emit_event(NFTransferred { from_acc, to_acc, token_id });
            Ok(())
        }

        /// Burn a token you own.
        #[ink(message)]
        pub fn burn(&mut self, token_id: TokenId) -> Result<()> {
            let from_acc = self
                .owner_of
                .get(&token_id)
                .ok_or(Error::TokenMissing)?;
            if from_acc != self.env().caller() {
                return Err(Error::NotOwner)
            }

            self.remove_token_from_owner(from_acc, token_id)?;
            self.owner_of.remove(&token_id);

            self.env().emit_event(NFBurned { from_acc, token_id });
            Ok(())
        }

        // --------- queries ---------

        /// Who owns this token?
        #[ink(message)]
        pub fn owner_of(&self, token_id: TokenId) -> Option<AccountId> {
            self.owner_of.get(&token_id)
        }

        /// How many tokens does this account own?
        #[ink(message)]
        pub fn balance_of(&self, owner_acc: AccountId) -> u32 {
            self.owned_count.get(&owner_acc).unwrap_or(0)
        }

        /// Paginated list of token ids owned by `owner_acc`.
        #[ink(message)]
        pub fn tokens_of(&self, owner_acc: AccountId, start_index: u32, limit: u32) -> Vec<TokenId> {
            let count = self.balance_of(owner_acc);
            if start_index >= count || limit == 0 {
                return Vec::new();
            }
            let end_index = min(count, start_index.saturating_add(limit));
            let mut list: Vec<TokenId> = Vec::new();
            let mut idx = start_index;
            while idx < end_index {
                if let Some(token_id) = self.tokens_by_owner.get(&(owner_acc, idx)) {
                    list.push(token_id);
                }
                idx += 1;
            }
            list
        }

        // --------- internals: owner sets management ---------

        fn add_token_to_owner(&mut self, to_acc: AccountId, token_id: TokenId) -> Result<()> {
            let count = self.owned_count.get(&to_acc).unwrap_or(0);
            self.tokens_by_owner.insert(&(to_acc, count), &token_id);
            self.owned_index.insert(&token_id, &count);
            self.owned_count.insert(&to_acc, &(count.checked_add(1).ok_or(Error::Overflow)?));
            Ok(())
        }

        fn remove_token_from_owner(&mut self, from_acc: AccountId, token_id: TokenId) -> Result<()> {
            let count = self.owned_count.get(&from_acc).unwrap_or(0);
            if count == 0 {
                return Err(Error::TokenMissing)
            }

            // index of token to remove
            let idx = self.owned_index.get(&token_id).ok_or(Error::TokenMissing)?;

            // last token info
            let last_idx = count - 1;
            if let Some(last_token_id) = self.tokens_by_owner.get(&(from_acc, last_idx)) {
                // move last token into the removed slot if not the same token
                if last_idx != idx {
                    self.tokens_by_owner.insert(&(from_acc, idx), &last_token_id);
                    self.owned_index.insert(&last_token_id, &idx);
                }
                // clear last slot
                self.tokens_by_owner.remove(&(from_acc, last_idx));
            }

            // clear mappings for removed token
            self.owned_index.remove(&token_id);

            // decrement count
            self.owned_count.insert(&from_acc, &(last_idx));

            Ok(())
        }
    }
}
