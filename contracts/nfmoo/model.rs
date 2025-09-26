use ink::storage::Mapping;

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
    pub(crate) to_acc: AccountId,
    #[ink(topic)]
    pub(crate) token_id: TokenId,
}

#[ink(event)]
pub struct NFTransferred {
    #[ink(topic)]
    pub(crate) from_acc: AccountId,
    #[ink(topic)]
    pub(crate) to_acc: AccountId,
    #[ink(topic)]
    pub(crate) token_id: TokenId,
}

#[ink(event)]
pub struct NFBurned {
    #[ink(topic)]
    pub(crate) from_acc: AccountId,
    #[ink(topic)]
    pub(crate) token_id: TokenId,
}

#[ink(event)]
pub struct NFApproval {
    #[ink(topic)]
    pub(crate) owner_acc: AccountId,
    #[ink(topic)]
    pub(crate) approved_acc: AccountId,
    #[ink(topic)]
    pub(crate) token_id: TokenId,
}

#[ink(event)]
pub struct NFApprovalForAll {
    #[ink(topic)]
    pub(crate) owner_acc: AccountId,
    #[ink(topic)]
    pub(crate) operator_acc: AccountId,
    pub(crate) approved_flag: bool,
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
pub struct NFMoo {
    // governance / roles
    pub(crate) owner_acc: AccountId,
    pub(crate) paused_flag: bool,
    pub(crate) is_minter: Mapping<AccountId, bool>,

    // supply controls
    pub(crate) max_supply_opt: Option<u128>,
    pub(crate) supply_cnt: u128,

    // enumeration
    pub(crate) next_id: TokenId,                       // next token id to assign on mint
    pub(crate) owner_by_id: Mapping<TokenId, AccountId>, // token_id -> owner_acc
    pub(crate) owned_count: Mapping<AccountId, u32>,     // owner_acc -> number of tokens owned
    pub(crate) tokens_by_owner: Mapping<(AccountId, u32), TokenId>, // (owner_acc, index_val) -> token_id
    pub(crate) owned_index: Mapping<TokenId, u32>,        // token_id -> index within owner's list

    // approvals
    pub(crate) token_approval: Mapping<TokenId, AccountId>,
    pub(crate) operator_approval: Mapping<(AccountId, AccountId), bool>,

    // versioning
    pub(crate) storage_ver_u32: u32,
}
