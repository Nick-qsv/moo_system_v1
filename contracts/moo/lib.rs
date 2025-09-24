#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod moo {
    use ink::storage::Mapping;

    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(scale::Encode, scale::Decode, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        AmountZero,
        InsufficientBalance,
        InsufficientAllowance,
        Overflow,
        SameAccount,
    }

    #[ink(event)]
    pub struct Transferred {
        #[ink(topic)]
        from_acc: AccountId,
        #[ink(topic)]
        to_acc: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct Minted {
        #[ink(topic)]
        to_acc: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct Approved {
        #[ink(topic)]
        owner_acc: AccountId,
        #[ink(topic)]
        spender_acc: AccountId,
        amount: Balance,
    }

    #[ink(storage)]
    pub struct Moo {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    impl Moo {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                total_supply: 0,
                balances: Mapping::default(),
                allowances: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, owner_acc: AccountId) -> Balance {
            self.balances.get(&owner_acc).unwrap_or(0)
        }

        #[ink(message)]
        pub fn my_balance(&self) -> Balance {
            let caller_acc = self.env().caller();
            self.balance_of(caller_acc)
        }

        #[ink(message)]
        pub fn mint(&mut self, amount: Balance) -> Result<()> {
            if amount == 0 {
                return Err(Error::AmountZero)
            }
            let caller_acc = self.env().caller();
            self.mint_internal(caller_acc, amount)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to_acc: AccountId, amount: Balance) -> Result<()> {
            if amount == 0 {
                return Err(Error::AmountZero)
            }
            let from_acc = self.env().caller();
            if from_acc == to_acc {
                return Err(Error::SameAccount)
            }
            self.move_balance(from_acc, to_acc, amount)
        }

        #[ink(message)]
        pub fn approve(&mut self, spender_acc: AccountId, amount: Balance) -> Result<()> {
            let owner_acc = self.env().caller();
            self.allowances.insert(&(owner_acc, spender_acc), &amount);
            self.env().emit_event(Approved { owner_acc, spender_acc, amount });
            Ok(())
        }

        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from_acc: AccountId,
            to_acc: AccountId,
            amount: Balance,
        ) -> Result<()> {
            if amount == 0 {
                return Err(Error::AmountZero)
            }
            if from_acc == to_acc {
                return Err(Error::SameAccount)
            }

            let caller_acc = self.env().caller();
            let mut allowance_amt = self.allowances.get(&(from_acc, caller_acc)).unwrap_or(0);
            if allowance_amt < amount {
                return Err(Error::InsufficientAllowance)
            }
            allowance_amt = allowance_amt.checked_sub(amount).ok_or(Error::Overflow)?;
            self.allowances.insert(&(from_acc, caller_acc), &allowance_amt);

            self.move_balance(from_acc, to_acc, amount)
        }

        #[ink(message)]
        pub fn allowance(&self, owner_acc: AccountId, spender_acc: AccountId) -> Balance {
            self.allowances.get(&(owner_acc, spender_acc)).unwrap_or(0)
        }

        // ---- internals ----

        fn mint_internal(&mut self, to_acc: AccountId, amount: Balance) -> Result<()> {
            let new_total = self.total_supply.checked_add(amount).ok_or(Error::Overflow)?;
            self.total_supply = new_total;

            let bal = self.balances.get(&to_acc).unwrap_or(0);
            let new_bal = bal.checked_add(amount).ok_or(Error::Overflow)?;
            self.balances.insert(&to_acc, &new_bal);

            self.env().emit_event(Minted { to_acc, amount });
            Ok(())
        }

        fn move_balance(&mut self, from_acc: AccountId, to_acc: AccountId, amount: Balance) -> Result<()> {
            let from_bal = self.balances.get(&from_acc).unwrap_or(0);
            if from_bal < amount {
                return Err(Error::InsufficientBalance)
            }
            let new_from = from_bal.checked_sub(amount).ok_or(Error::Overflow)?;
            self.balances.insert(&from_acc, &new_from);

            let to_bal = self.balances.get(&to_acc).unwrap_or(0);
            let new_to = to_bal.checked_add(amount).ok_or(Error::Overflow)?;
            self.balances.insert(&to_acc, &new_to);

            self.env().emit_event(Transferred { from_acc, to_acc, amount });
            Ok(())
        }
    }
}
