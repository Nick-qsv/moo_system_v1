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
    Unauthorized,
    Paused,
    NotOwner,
    AllowanceRace,
}

#[ink(event)]
pub struct Transferred {
    #[ink(topic)]
    pub(crate) from_acc: AccountId,
    #[ink(topic)]
    pub(crate) to_acc: AccountId,
    pub(crate) amount_val: Balance,
}

#[ink(event)]
pub struct Minted {
    #[ink(topic)]
    pub(crate) to_acc: AccountId,
    pub(crate) amount_val: Balance,
}

#[ink(event)]
pub struct Burned {
    #[ink(topic)]
    pub(crate) from_acc: AccountId,
    pub(crate) amount_val: Balance,
}

#[ink(event)]
pub struct Approved {
    #[ink(topic)]
    pub(crate) owner_acc: AccountId,
    #[ink(topic)]
    pub(crate) spender_acc: AccountId,
    pub(crate) amount_val: Balance,
}

#[ink(event)]
pub struct PausedSet {
    pub(crate) paused_flag: bool,
}

#[ink(event)]
pub struct MinterSet {
    #[ink(topic)]
    pub(crate) minter_acc: AccountId,
    pub(crate) enabled_flag: bool,
}

#[ink(storage)]
pub struct Moo {
    // governance / control
    pub(crate) owner_acc: AccountId,
    pub(crate) paused_flag: bool,
    pub(crate) is_minter: Mapping<AccountId, bool>,

    // token state
    pub(crate) total_supply: Balance,
    pub(crate) balances: Mapping<AccountId, Balance>,
    pub(crate) allowances: Mapping<(AccountId, AccountId), Balance>,

    // versioning (future migrations)
    pub(crate) storage_ver_u32: u32,
}

