NFMoo — Non‑Fungible Token (ink!)

Overview
- NFT with enumeration and approvals, implemented with ink! 5.x.
- Storage/events live in `model.rs`; constructors and messages in `logic.rs`.
- `lib.rs` declares the `#[ink::contract]` module and inlines both files so `impl NFMoo` resolves in the same scope.

Layout
- `model.rs` — storage, errors, and events
- `logic.rs` — constructors, admin/role controls, mint/transfer/burn, approvals, queries
- `lib.rs` — contract module wrapper, feature gating, optional re‑exports

Build & Test
- With cargo‑contract installed: `cargo contract build` from this directory.
- Or type‑check only: `cargo check --features std` (host build).
- For cross‑contract calls, enable `ink-as-dependency` to use `NFMooRef`.

