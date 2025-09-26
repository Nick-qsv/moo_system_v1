impl Moo {
    // -------- constructors --------

    #[ink(constructor)]
    pub fn new() -> Self {
        Self {
            owner_acc: Self::env().caller(),
            paused_flag: false,
            is_minter: Default::default(),
            total_supply: 0,
            balances: Default::default(),
            allowances: Default::default(),
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

    // -------- read API --------

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
    pub fn allowance(&self, owner_acc: AccountId, spender_acc: AccountId) -> Balance {
        self.allowances.get(&(owner_acc, spender_acc)).unwrap_or(0)
    }

    // -------- write API --------

    /// Privileged mint: caller must be marked as a minter.
    #[ink(message)]
    pub fn mint(&mut self, amount_val: Balance) -> Result<()> {
        self.when_not_paused()?;
        if amount_val == 0 {
            return Err(Error::AmountZero)
        }
        let caller_acc = self.env().caller();
        let allowed_flag = self.is_minter.get(&caller_acc).unwrap_or(false);
        if !allowed_flag {
            return Err(Error::Unauthorized)
        }
        self.mint_internal(caller_acc, amount_val)
    }

    #[ink(message)]
    pub fn burn(&mut self, amount_val: Balance) -> Result<()> {
        self.when_not_paused()?;
        if amount_val == 0 {
            return Err(Error::AmountZero)
        }
        let from_acc = self.env().caller();
        let from_bal = self.balances.get(&from_acc).unwrap_or(0);
        if from_bal < amount_val {
            return Err(Error::InsufficientBalance)
        }
        let new_from_bal = from_bal.checked_sub(amount_val).ok_or(Error::Overflow)?;
        self.balances.insert(&from_acc, &new_from_bal);
        self.total_supply = self.total_supply.checked_sub(amount_val).ok_or(Error::Overflow)?;
        self.env().emit_event(Burned { from_acc, amount_val });
        Ok(())
    }

    #[ink(message)]
    pub fn transfer(&mut self, to_acc: AccountId, amount_val: Balance) -> Result<()> {
        self.when_not_paused()?;
        if amount_val == 0 {
            return Err(Error::AmountZero)
        }
        let from_acc = self.env().caller();
        if from_acc == to_acc {
            return Err(Error::SameAccount)
        }
        self.move_balance(from_acc, to_acc, amount_val)
    }

    #[ink(message)]
    pub fn approve(&mut self, spender_acc: AccountId, amount_val: Balance) -> Result<()> {
        self.when_not_paused()?;
        let owner_acc = self.env().caller();
        let current_val = self.allowances.get(&(owner_acc, spender_acc)).unwrap_or(0);
        // Safe-approve: forbid nonzero -> nonzero without zeroing first
        if current_val != 0 && amount_val != 0 {
            return Err(Error::AllowanceRace)
        }
        self.allowances.insert(&(owner_acc, spender_acc), &amount_val);
        self.env().emit_event(Approved { owner_acc, spender_acc, amount_val });
        Ok(())
    }

    #[ink(message)]
    pub fn increase_allowance(&mut self, spender_acc: AccountId, add_val: Balance) -> Result<()> {
        self.when_not_paused()?;
        let owner_acc = self.env().caller();
        let current_val = self.allowances.get(&(owner_acc, spender_acc)).unwrap_or(0);
        let new_val = current_val.checked_add(add_val).ok_or(Error::Overflow)?;
        self.allowances.insert(&(owner_acc, spender_acc), &new_val);
        self.env().emit_event(Approved { owner_acc, spender_acc, amount_val: new_val });
        Ok(())
    }

    #[ink(message)]
    pub fn decrease_allowance(&mut self, spender_acc: AccountId, sub_val: Balance) -> Result<()> {
        self.when_not_paused()?;
        let owner_acc = self.env().caller();
        let current_val = self.allowances.get(&(owner_acc, spender_acc)).unwrap_or(0);
        let new_val = current_val.saturating_sub(sub_val);
        self.allowances.insert(&(owner_acc, spender_acc), &new_val);
        self.env().emit_event(Approved { owner_acc, spender_acc, amount_val: new_val });
        Ok(())
    }

    #[ink(message)]
    pub fn transfer_from(
        &mut self,
        from_acc: AccountId,
        to_acc: AccountId,
        amount_val: Balance,
    ) -> Result<()> {
        self.when_not_paused()?;
        if amount_val == 0 {
            return Err(Error::AmountZero)
        }
        if from_acc == to_acc {
            return Err(Error::SameAccount)
        }

        // Precheck balances to avoid burning allowance on failure
        let from_bal = self.balances.get(&from_acc).unwrap_or(0);
        if from_bal < amount_val {
            return Err(Error::InsufficientBalance)
        }

        // Check allowance
        let caller_acc = self.env().caller();
        let current_allow = self.allowances.get(&(from_acc, caller_acc)).unwrap_or(0);
        if current_allow < amount_val {
            return Err(Error::InsufficientAllowance)
        }

        // Move balances (overflow-safe)
        let to_bal = self.balances.get(&to_acc).unwrap_or(0);
        let new_from = from_bal.checked_sub(amount_val).ok_or(Error::Overflow)?;
        let new_to = to_bal.checked_add(amount_val).ok_or(Error::Overflow)?;
        self.balances.insert(&from_acc, &new_from);
        self.balances.insert(&to_acc, &new_to);
        self.env().emit_event(Transferred { from_acc, to_acc, amount_val });

        // Reduce allowance last
        let new_allow = current_allow - amount_val;
        self.allowances.insert(&(from_acc, caller_acc), &new_allow);
        Ok(())
    }

    // ---- internals ----

    fn mint_internal(&mut self, to_acc: AccountId, amount_val: Balance) -> Result<()> {
        let new_total = self.total_supply.checked_add(amount_val).ok_or(Error::Overflow)?;
        self.total_supply = new_total;

        let to_bal = self.balances.get(&to_acc).unwrap_or(0);
        let new_to = to_bal.checked_add(amount_val).ok_or(Error::Overflow)?;
        self.balances.insert(&to_acc, &new_to);

        self.env().emit_event(Minted { to_acc, amount_val });
        Ok(())
    }

    fn move_balance(&mut self, from_acc: AccountId, to_acc: AccountId, amount_val: Balance) -> Result<()> {
        let from_bal = self.balances.get(&from_acc).unwrap_or(0);
        if from_bal < amount_val {
            return Err(Error::InsufficientBalance)
        }
        let new_from = from_bal.checked_sub(amount_val).ok_or(Error::Overflow)?;
        self.balances.insert(&from_acc, &new_from);

        let to_bal = self.balances.get(&to_acc).unwrap_or(0);
        let new_to = to_bal.checked_add(amount_val).ok_or(Error::Overflow)?;
        self.balances.insert(&to_acc, &new_to);

        self.env().emit_event(Transferred { from_acc, to_acc, amount_val });
        Ok(())
    }
}
