//! Anchor ⚓ is a framework for Solana's Sealevel runtime providing several
//! convenient developer tools.
//!
//! - Rust eDSL for writing safe, secure, and high level Solana programs
//! - [IDL](https://en.wikipedia.org/wiki/Interface_description_language) specification
//! - TypeScript package for generating clients from IDL
//! - CLI and workspace management for developing complete applications
//!
//! If you're familiar with developing in Ethereum's
//! [Solidity](https://docs.soliditylang.org/en/v0.7.4/),
//! [Truffle](https://www.trufflesuite.com/),
//! [web3.js](https://github.com/ethereum/web3.js) or Parity's
//! [Ink!](https://github.com/paritytech/ink), then the experience will be
//! familiar. Although the syntax and semantics are targeted at Solana, the high
//! level workflow of writing RPC request handlers, emitting an IDL, and
//! generating clients from IDL is the same.
//!
//! For detailed tutorials and examples on how to use Anchor, see the guided
//! [tutorials](https://project-serum.github.io/anchor) or examples in the GitHub
//! [repository](https://github.com/project-serum/anchor).
//!
//! Presented here are the Rust primitives for building on Solana.

extern crate self as anchor_lang;

use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::io::Write;

mod account_meta;
pub mod accounts;
mod bpf_upgradeable_state;
mod common;
pub mod context;
mod ctor;
mod error;
#[doc(hidden)]
pub mod idl;
mod system_program;

pub use crate::system_program::System;
mod vec;
pub use crate::bpf_upgradeable_state::*;
pub use anchor_attribute_access_control::access_control;
pub use anchor_attribute_account::{account, declare_id, zero_copy};
pub use anchor_attribute_constant::constant;
pub use anchor_attribute_error::error;
pub use anchor_attribute_event::{emit, event};
pub use anchor_attribute_interface::interface;
pub use anchor_attribute_program::program;
pub use anchor_attribute_state::state;
pub use anchor_derive_accounts::Accounts;
/// Borsh is the default serialization format for instructions and accounts.
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use solana_program;

/// A data structure of validated accounts that can be deserialized from the
/// input to a Solana program. Implementations of this trait should perform any
/// and all requisite constraint checks on accounts to ensure the accounts
/// maintain any invariants required for the program to run securely. In most
/// cases, it's recommended to use the [`Accounts`](./derive.Accounts.html)
/// derive macro to implement this trait.
pub trait Accounts<'info>: ToAccountMetas + ToAccountInfos<'info> + Sized {
    /// Returns the validated accounts struct. What constitutes "valid" is
    /// program dependent. However, users of these types should never have to
    /// worry about account substitution attacks. For example, if a program
    /// expects a `Mint` account from the SPL token program  in a particular
    /// field, then it should be impossible for this method to return `Ok` if
    /// any other account type is given--from the SPL token program or elsewhere.
    ///
    /// `program_id` is the currently executing program. `accounts` is the
    /// set of accounts to construct the type from. For every account used,
    /// the implementation should mutate the slice, consuming the used entry
    /// so that it cannot be used again.
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        ix_data: &[u8],
    ) -> Result<Self, ProgramError>;
}

/// The exit procedure for an account. Any cleanup or persistence to storage
/// should be done here.
pub trait AccountsExit<'info>: ToAccountMetas + ToAccountInfos<'info> {
    /// `program_id` is the currently executing program.
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        // no-op
        Ok(())
    }
}

/// The close procedure to initiate garabage collection of an account, allowing
/// one to retrieve the rent exemption.
pub trait AccountsClose<'info>: ToAccountInfos<'info> {
    fn close(&self, sol_destination: AccountInfo<'info>) -> ProgramResult;
}

/// Transformation to
/// [`AccountMeta`](../solana_program/instruction/struct.AccountMeta.html)
/// structs.
pub trait ToAccountMetas {
    /// `is_signer` is given as an optional override for the signer meta field.
    /// This covers the edge case when a program-derived-address needs to relay
    /// a transaction from a client to another program but sign the transaction
    /// before the relay. The client cannot mark the field as a signer, and so
    /// we have to override the is_signer meta field given by the client.
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta>;
}

/// Transformation to
/// [`AccountInfo`](../solana_program/account_info/struct.AccountInfo.html)
/// structs.
pub trait ToAccountInfos<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;
}

/// Transformation to an `AccountInfo` struct.
pub trait ToAccountInfo<'info> {
    fn to_account_info(&self) -> AccountInfo<'info>;
}

impl<'info, T> ToAccountInfo<'info> for T
where
    T: AsRef<AccountInfo<'info>>,
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.as_ref().clone()
    }
}

/// A data structure that can be serialized and stored into account storage,
/// i.e. an
/// [`AccountInfo`](../solana_program/account_info/struct.AccountInfo.html#structfield.data)'s
/// mutable data slice.
///
/// Implementors of this trait should ensure that any subsequent usage of the
/// `AccountDeserialize` trait succeeds if and only if the account is of the
/// correct type.
///
/// In most cases, one can use the default implementation provided by the
/// [`#[account]`](./attr.account.html) attribute.
pub trait AccountSerialize {
    /// Serializes the account data into `writer`.
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        Ok(())
    }
}

/// A data structure that can be deserialized and stored into account storage,
/// i.e. an
/// [`AccountInfo`](../solana_program/account_info/struct.AccountInfo.html#structfield.data)'s
/// mutable data slice.
pub trait AccountDeserialize: Sized {
    /// Deserializes previously initialized account data. Should fail for all
    /// uninitialized accounts, where the bytes are zeroed. Implementations
    /// should be unique to a particular account type so that one can never
    /// successfully deserialize the data of one account type into another.
    /// For example, if the SPL token program were to implement this trait,
    /// it should be impossible to deserialize a `Mint` account into a token
    /// `Account`.
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Self::try_deserialize_unchecked(buf)
    }

    /// Deserializes account data without checking the account discriminator.
    /// This should only be used on account initialization, when the bytes of
    /// the account are zeroed.
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError>;
}

/// An account data structure capable of zero copy deserialization.
pub trait ZeroCopy: Discriminator + Copy + Clone + Zeroable + Pod {}

/// Calculates the data for an instruction invocation, where the data is
/// `Sha256(<namespace>::<method_name>)[..8] || BorshSerialize(args)`.
/// `args` is a borsh serialized struct of named fields for each argument given
/// to an instruction.
pub trait InstructionData: AnchorSerialize {
    fn data(&self) -> Vec<u8>;
}

/// An event that can be emitted via a Solana log.
pub trait Event: AnchorSerialize + AnchorDeserialize + Discriminator {
    fn data(&self) -> Vec<u8>;
}

// The serialized event data to be emitted via a Solana log.
// TODO: remove this on the next major version upgrade.
#[doc(hidden)]
#[deprecated(since = "0.4.2", note = "Please use Event instead")]
pub trait EventData: AnchorSerialize + Discriminator {
    fn data(&self) -> Vec<u8>;
}

/// 8 byte unique identifier for a type.
pub trait Discriminator {
    fn discriminator() -> [u8; 8];
}

/// Bump seed for program derived addresses.
pub trait Bump {
    fn seed(&self) -> u8;
}

/// Defines an address expected to own an account.
pub trait Owner {
    fn owner() -> Pubkey;
}

/// Defines the id of a program.
pub trait Id {
    fn id() -> Pubkey;
}

/// Defines the Pubkey of an account.
pub trait Key {
    fn key(&self) -> Pubkey;
}

impl<'info, T> Key for T
where
    T: AsRef<AccountInfo<'info>>,
{
    fn key(&self) -> Pubkey {
        *self.as_ref().key
    }
}

/// The prelude contains all commonly used components of the crate.
/// All programs should include it via `anchor_lang::prelude::*;`.
pub mod prelude {
    pub use super::{
        access_control, account, accounts::account::Account,
        accounts::loader_account::AccountLoader, accounts::program::Program,
        accounts::signer::Signer, accounts::system_account::SystemAccount,
        accounts::sysvar::Sysvar, accounts::unchecked_account::UncheckedAccount, constant,
        context::Context, context::CpiContext, declare_id, emit, error, event, interface, program,
        require, solana_program::bpf_loader_upgradeable::UpgradeableLoaderState, state, zero_copy,
        AccountDeserialize, AccountSerialize, Accounts, AccountsExit, AnchorDeserialize,
        AnchorSerialize, Id, Key, Owner, ProgramData, System, ToAccountInfo, ToAccountInfos,
        ToAccountMetas,
    };

    pub use borsh;
    pub use solana_program::account_info::{next_account_info, AccountInfo};
    pub use solana_program::entrypoint::ProgramResult;
    pub use solana_program::instruction::AccountMeta;
    pub use solana_program::msg;
    pub use solana_program::program_error::ProgramError;
    pub use solana_program::pubkey::Pubkey;
    pub use solana_program::sysvar::clock::Clock;
    pub use solana_program::sysvar::epoch_schedule::EpochSchedule;
    pub use solana_program::sysvar::fees::Fees;
    pub use solana_program::sysvar::instructions::Instructions;
    pub use solana_program::sysvar::recent_blockhashes::RecentBlockhashes;
    pub use solana_program::sysvar::rent::Rent;
    pub use solana_program::sysvar::rewards::Rewards;
    pub use solana_program::sysvar::slot_hashes::SlotHashes;
    pub use solana_program::sysvar::slot_history::SlotHistory;
    pub use solana_program::sysvar::stake_history::StakeHistory;
    pub use solana_program::sysvar::Sysvar as SolanaSysvar;
    pub use thiserror;
}

/// Internal module used by macros and unstable apis.
pub mod __private {
    // Modules with useful information for users
    // don't use #[doc(hidden)] on these
    pub use crate::error::ErrorCode;

    /// The discriminator anchor uses to mark an account as closed.
    pub const CLOSED_ACCOUNT_DISCRIMINATOR: [u8; 8] = [255, 255, 255, 255, 255, 255, 255, 255];

    /// The starting point for user defined error codes.
    pub const ERROR_CODE_OFFSET: u32 = 6000;

    #[doc(hidden)]
    pub use crate::ctor::Ctor;
    #[doc(hidden)]
    pub use crate::error::Error;
    #[doc(hidden)]
    pub use anchor_attribute_account::ZeroCopyAccessor;
    #[doc(hidden)]
    pub use anchor_attribute_event::EventIndex;
    #[doc(hidden)]
    pub use base64;
    #[doc(hidden)]
    pub use bytemuck;
    #[doc(hidden)]
    use solana_program::program_error::ProgramError;
    #[doc(hidden)]
    use solana_program::pubkey::Pubkey;
    #[doc(hidden)]
    #[doc(hidden)]
    pub mod state {
        pub use crate::accounts::state::*;
    }

    // Calculates the size of an account, which may be larger than the deserialized
    // data in it. This trait is currently only used for `#[state]` accounts.
    #[doc(hidden)]
    pub trait AccountSize {
        fn size(&self) -> Result<u64, ProgramError>;
    }

    // Very experimental trait.
    #[doc(hidden)]
    pub trait ZeroCopyAccessor<Ty> {
        fn get(&self) -> Ty;
        fn set(input: &Ty) -> Self;
    }

    #[doc(hidden)]
    impl ZeroCopyAccessor<Pubkey> for [u8; 32] {
        fn get(&self) -> Pubkey {
            Pubkey::new(self)
        }
        fn set(input: &Pubkey) -> [u8; 32] {
            input.to_bytes()
        }
    }

    #[doc(hidden)]
    pub use crate::accounts::state::PROGRAM_STATE_SEED;
}

/// Ensures a condition is true, otherwise returns the given error.
/// Use this with a custom error type.
///
/// # Example
/// ```ignore
/// // Instruction function
/// pub fn set_data(ctx: Context<SetData>, data: u64) -> ProgramResult {
///     require!(ctx.accounts.data.mutation_allowed, MyError::MutationForbidden);
///     ctx.accounts.data.data = data;
///     Ok(())
/// }
///
/// // An enum for custom error codes
/// #[error]
/// pub enum MyError {
///     MutationForbidden
/// }
///
/// // An account definition
/// #[account]
/// #[derive(Default)]
/// pub struct MyData {
///     mutation_allowed: bool,
///     data: u64
/// }
///
/// // An account validation struct
/// #[derive(Accounts)]
/// pub struct SetData<'info> {
///     #[account(mut)]
///     pub data: Account<'info, MyData>
/// }
/// ```
#[macro_export]
macro_rules! require {
    ($invariant:expr, $error:tt $(,)?) => {
        if !($invariant) {
            return Err(crate::ErrorCode::$error.into());
        }
    };
    ($invariant:expr, $error:expr $(,)?) => {
        if !($invariant) {
            return Err($error.into());
        }
    };
}
