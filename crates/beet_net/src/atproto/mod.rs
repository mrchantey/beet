//! An account's repo as a provider: the [`Pds`] family and the converge that
//! makes a repo match what a document declares.
//!
//! The erased-provider pattern: a [`PdsProvider`] trait, the [`Pds`] handle
//! every consumer clones, the `<AtprotoAccount/>` declaration whose attach
//! lands one on its entity, and [`PdsQuery`] to resolve it. Two providers:
//!
//! - [`EmulatorPds`], a repo laid out in any [`BlobStore`] that performs a
//!   PDS's duties on write, for tests and for a repo that never leaves the
//!   machine. Always compiled: the data model needs no network.
//! - `XrpcPds`, a real PDS over xrpc through the `AtprotoAuth` credential
//!   seam, with did and handle resolution, behind the `atproto` feature.
//!
//! A record body crosses as an `AtprotoValue`, sealed in the data model, which
//! is how a float reaches a repo that has none, and a record type is its body
//! alone: the rkey it lives at travels beside it, in the uri a read answers
//! and the `Rkeyed` a write takes. The `app.bsky.*` lexicons beet writes
//! (`FeedPost`, `RichText`) are the `bsky` module's, behind `atproto`.
//!
//! The protocol's primitives (`Did`, `Rkey`, `Tid`, `Cid`, `StrongRef`,
//! `BlobRef`) and their words are `beet_core::atproto`'s.
//!
//! # Words
//!
//! - **Natural key.** The field that identifies a record without any stored
//!   id: a publication's `url`, a standard site document's `path`, an
//!   announcement's `embed.external.uri`, a repost's `subject.uri`. It keys
//!   the index remembering which rkey each record was written at, and it is
//!   what a listing matches a record by when that index has to be rebuilt.
//! - **Converge.** For each declared record: read it at the rkey the index
//!   remembers, compare, then write or report, so the repo comes to match the
//!   declaration ([`Pds::converge`]). A collection is listed only to find
//!   orphans or rebuild a lost index. The atproto twin of a tofu apply, and
//!   why running a publish twice writes nothing the second time.
//!
//! # Live tests
//!
//! A test that writes to a real PDS needs a throwaway account, created by
//! hand once, never one we publish from:
//!
//! 1. Sign up at `bsky.app` with a `*.bsky.social` handle and nothing else
//!    filled in.
//! 2. Settings -> Privacy and security -> App passwords, add one.
//! 3. From the beet checkout, seal all three into the `agents` group of its
//!    `secrets.toml`, which the test runner loads into the process
//!    environment:
//!
//! ```sh
//! beet secrets/set BEET_TEST_ATPROTO_HANDLE --group=agents --role=env_var --note="throwaway bsky.social account for live write tests"
//! beet secrets/set BEET_TEST_ATPROTO_DID --group=agents --role=env_var --note="the did of that account, so a test needs no resolution to start"
//! beet secrets/set BEET_TEST_ATPROTO_APP_PASSWORD --group=agents --role=env_var --note="app password for that account" --rotation="manual:https://bsky.app/settings/app-passwords"
//! ```
//!
//! A live write test reads the three and skips with a note naming them when
//! any is absent, so a fresh checkout passes without the account. The did is
//! stored rather than resolved because a handle can be renamed and a test
//! should not depend on a lookup to decide whether to run.
mod atproto_account;
mod atproto_plugin;
mod converge;
pub mod dag_cbor_ext;
mod emulator_pds;
mod pds;
mod pds_query;
pub use atproto_account::*;
pub use atproto_plugin::*;
pub use converge::*;
pub use emulator_pds::*;
pub use pds::*;
pub use pds_query::*;
#[cfg(feature = "atproto")]
mod atproto_auth;
#[cfg(feature = "atproto")]
mod bsky;
#[cfg(feature = "atproto")]
mod identity;
#[cfg(all(
	test,
	feature = "atproto",
	any(feature = "ureq", feature = "reqwest"),
	not(target_arch = "wasm32")
))]
mod live_test;
#[cfg(feature = "atproto")]
mod xrpc_pds;
#[cfg(feature = "atproto")]
pub use atproto_auth::*;
#[cfg(feature = "atproto")]
pub use bsky::*;
#[cfg(feature = "atproto")]
pub use identity::*;
#[cfg(feature = "atproto")]
pub use xrpc_pds::*;
