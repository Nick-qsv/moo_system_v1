#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod nfmoo {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
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
        NotApproved,
        TokenMissing,
        Unauthorized,
        Paused,
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

    #[ink(event)]
    pub struct NFApproval {
        #[ink(topic)]
        owner_acc: AccountId,
        #[ink(topic)]
        approved_acc: AccountId,
        #[ink(topic)]
        token_id: TokenId,
    }

    #[ink(event)]
    pub struct NFApprovalForAll {
        #[ink(topic)]
        owner_acc: AccountId,
        #[ink(topic)]
        operator_acc: AccountId,
        approved_flag: bool,
    }

    #[ink(event)]
    pub struct PausedSet {
        paused_flag: bool,
    }

    #[ink(event)]
    pub struct MinterSet {
        #[ink(topic)]
        minter_acc: AccountId,
        enabled_flag: bool,
    }

    #[ink(storage)]
    pub struct NFMoo {
        // governance / roles
        owner_acc: AccountId,
        paused_flag: bool,
        is_minter: Mapping<AccountId, bool>,

        // supply controls
        max_supply_opt: Option<u128>,
        supply_cnt: u128,

        // enumeration
        next_id: TokenId,                       // next token id to assign on mint
        owner_by_id: Mapping<TokenId, AccountId>, // token_id -> owner_acc
        owned_count: Mapping<AccountId, u32>,     // owner_acc -> number of tokens owned
        tokens_by_owner: Mapping<(AccountId, u32), TokenId>, // (owner_acc, index_val) -> token_id
        owned_index: Mapping<TokenId, u32>,        // token_id -> index within owner's list

        // approvals
        token_approval: Mapping<TokenId, AccountId>,
        operator_approval: Mapping<(AccountId, AccountId), bool>,

        // versioning
        storage_ver_u32: u32,
    }

    impl NFMoo {
        // -------- constructors --------

        #[ink(constructor)]
        pub fn new(max_supply_opt: Option<u128>) -> Self {
            Self {
                owner_acc: Self::env().caller(),
                paused_flag: false,
                is_minter: Mapping::default(),
                max_supply_opt,
                supply_cnt: 0,
                next_id: 0,
                owner_by_id: Mapping::default(),
                owned_count: Mapping::default(),
                tokens_by_owner: Mapping::default(),
                owned_index: Mapping::default(),
                token_approval: Mapping::default(),
                operator_approval: Mapping::default(),
                storage_ver_u32: 1,
            }
        }

        // -------- modifiers (helpers) --------

        fn only_owner(&self) -> Result<()> {
            if self.env().caller() != self.owner_acc {
                return Err(Error::NotOwner)
            }
            Ok(())
        }

        fn when_not_paused(&self) -> Result<()> {
            if self.paused_flag {
                return Err(Error::Paused)
            }
            Ok(())
        }

        fn is_approved_or_owner(&self, caller_acc: AccountId, token_id: TokenId) -> Result<()> {
            let owner_acc = self.owner_by_id.get(&token_id).ok_or(Error::TokenMissing)?;
            if caller_acc == owner_acc {
                return Ok(())
            }
            if self.token_approval.get(&token_id) == Some(caller_acc) {
                return Ok(())
            }
            if self.operator_approval.get(&(owner_acc, caller_acc)).unwrap_or(false) {
                return Ok(())
            }
            Err(Error::NotApproved)
        }

        // -------- admin / roles --------

        #[ink(message)]
        pub fn set_pause(&mut self, paused_flag: bool) -> Result<()> {
            self.only_owner()?;
            self.paused_flag = paused_flag;
            self.env().emit_event(PausedSet { paused_flag });
            Ok(())
        }

        #[ink(message)]
        pub fn set_minter(&mut self, minter_acc: AccountId, enabled_flag: bool) -> Result<()> {
            self.only_owner()?;
            self.is_minter.insert(&minter_acc, &enabled_flag);
            self.env().emit_event(MinterSet { minter_acc, enabled_flag });
            Ok(())
        }

        // -------- mint / burn / transfer --------

        /// Privileged, bounded mint to caller (minter).
        #[ink(message)]
        pub fn mint_n(&mut self, amount_cnt: u32) -> Result<()> {
            self.when_not_paused()?;
            if amount_cnt == 0 {
                return Err(Error::AmountZero)
            }
            let caller_acc = self.env().caller();
            if !self.is_minter.get(&caller_acc).unwrap_or(false) {
                return Err(Error::Unauthorized)
            }

            const MAX_PER_CALL: u32 = 200;
            if amount_cnt > MAX_PER_CALL {
                return Err(Error::Overflow)
            }

            for _ in 0..amount_cnt {
                if let Some(max_supply_val) = self.max_supply_opt {
                    if self.supply_cnt >= max_supply_val {
                        return Err(Error::Overflow)
                    }
                }

                let token_id = self.next_id;
                self.next_id = self.next_id.checked_add(1).ok_or(Error::Overflow)?;

                self.owner_by_id.insert(&token_id, &caller_acc);
                self.add_token_to_owner(caller_acc, token_id)?;
                self.supply_cnt = self.supply_cnt.checked_add(1).ok_or(Error::Overflow)?;
                self.env().emit_event(NFMinted { to_acc: caller_acc, token_id });
            }
            Ok(())
        }

        /// Transfer a token (caller must be owner or approved).
        #[ink(message)]
        pub fn transfer(&mut self, to_acc: AccountId, token_id: TokenId) -> Result<()> {
            self.when_not_paused()?;
            let caller_acc = self.env().caller();
            self.is_approved_or_owner(caller_acc, token_id)?;
            let from_acc = self.owner_by_id.get(&token_id).ok_or(Error::TokenMissing)?;
            if from_acc == to_acc {
                return Err(Error::SameAccount)
            }

            self.clear_token_approval(token_id);
            self.remove_token_from_owner(from_acc, token_id)?;
            self.owner_by_id.insert(&token_id, &to_acc);
            self.add_token_to_owner(to_acc, token_id)?;

            self.env().emit_event(NFTransferred { from_acc, to_acc, token_id });
            Ok(())
        }

        /// Burn a token you own (no operator burn by default).
        #[ink(message)]
        pub fn burn(&mut self, token_id: TokenId) -> Result<()> {
            self.when_not_paused()?;
            let from_acc = self.owner_by_id.get(&token_id).ok_or(Error::TokenMissing)?;
            if from_acc != self.env().caller() {
                return Err(Error::NotOwner)
            }

            self.clear_token_approval(token_id);
            self.remove_token_from_owner(from_acc, token_id)?;
            self.owner_by_id.remove(&token_id);
            self.supply_cnt = self.supply_cnt.checked_sub(1).ok_or(Error::Overflow)?;
            self.env().emit_event(NFBurned { from_acc, token_id });
            Ok(())
        }

        // -------- approvals --------

        #[ink(message)]
        pub fn approve(&mut self, approved_acc: AccountId, token_id: TokenId) -> Result<()> {
            self.when_not_paused()?;
            let owner_acc = self.owner_by_id.get(&token_id).ok_or(Error::TokenMissing)?;
            if owner_acc != self.env().caller() {
                return Err(Error::NotOwner)
            }
            self.token_approval.insert(&token_id, &approved_acc);
            self.env().emit_event(NFApproval { owner_acc, approved_acc, token_id });
            Ok(())
        }

        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator_acc: AccountId, approved_flag: bool) -> Result<()> {
            self.when_not_paused()?;
            let owner_acc = self.env().caller();
            if owner_acc == operator_acc {
                return Err(Error::SameAccount)
            }
            self.operator_approval.insert(&(owner_acc, operator_acc), &approved_flag);
            self.env().emit_event(NFApprovalForAll { owner_acc, operator_acc, approved_flag });
            Ok(())
        }

        #[ink(message)]
        pub fn get_approved(&self, token_id: TokenId) -> Option<AccountId> {
            self.token_approval.get(&token_id)
        }

        #[ink(message)]
        pub fn is_approved_for_all(&self, owner_acc: AccountId, operator_acc: AccountId) -> bool {
            self.operator_approval.get(&(owner_acc, operator_acc)).unwrap_or(false)
        }

        // -------- queries --------

        /// Who owns this token?
        #[ink(message)]
        pub fn owner_of(&self, token_id: TokenId) -> Option<AccountId> {
            self.owner_by_id.get(&token_id)
        }

        /// How many tokens does this account own?
        #[ink(message)]
        pub fn balance_of(&self, owner_acc: AccountId) -> u32 {
            self.owned_count.get(&owner_acc).unwrap_or(0)
        }

        /// Paginated list of token ids owned by `owner_acc`.
        #[ink(message)]
        pub fn tokens_of(&self, owner_acc: AccountId, start_index: u32, limit_cnt: u32) -> Vec<TokenId> {
            let count_val = self.balance_of(owner_acc);
            if start_index >= count_val || limit_cnt == 0 {
                return Vec::new()
            }
            let end_index = min(count_val, start_index.saturating_add(limit_cnt));
            let mut list_vec: Vec<TokenId> = Vec::new();
            let mut index_val = start_index;
            while index_val < end_index {
                if let Some(token_id) = self.tokens_by_owner.get(&(owner_acc, index_val)) {
                    list_vec.push(token_id);
                }
                index_val += 1;
            }
            list_vec
        }

        // -------- internals: owner sets management --------

        fn add_token_to_owner(&mut self, to_acc: AccountId, token_id: TokenId) -> Result<()> {
            let count_val = self.owned_count.get(&to_acc).unwrap_or(0);
            self.tokens_by_owner.insert(&(to_acc, count_val), &token_id);
            self.owned_index.insert(&token_id, &count_val);
            let new_count = count_val.checked_add(1).ok_or(Error::Overflow)?;
            self.owned_count.insert(&to_acc, &new_count);
            Ok(())
        }

        fn remove_token_from_owner(&mut self, from_acc: AccountId, token_id: TokenId) -> Result<()> {
            let count_val = self.owned_count.get(&from_acc).unwrap_or(0);
            if count_val == 0 {
                return Err(Error::TokenMissing)
            }

            // index of token to remove
            let remove_index = self.owned_index.get(&token_id).ok_or(Error::TokenMissing)?;

            // last token info
            let last_index = count_val - 1;
            if let Some(last_token_id) = self.tokens_by_owner.get(&(from_acc, last_index)) {
                // move last token into the removed slot if not the same token
                if last_index != remove_index {
                    self.tokens_by_owner.insert(&(from_acc, remove_index), &last_token_id);
                    self.owned_index.insert(&last_token_id, &remove_index);
                }
                // clear last slot
                self.tokens_by_owner.remove(&(from_acc, last_index));
            }

            // clear mappings for removed token
            self.owned_index.remove(&token_id);

            // decrement count
            self.owned_count.insert(&from_acc, &last_index);

            Ok(())
        }

        fn clear_token_approval(&mut self, token_id: TokenId) {
            self.token_approval.remove(&token_id);
        }
    }
}