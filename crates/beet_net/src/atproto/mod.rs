//! An account's repo as a provider: the [`Pds`] family and the converge that
//! makes a repo match what a document declares.
//!
//! The erased-provider pattern: a [`PdsProvider`] trait, the [`Pds`] handle
//! every consumer clones, the `<AtprotoAccount/>` declaration whose attach
//! lands one on its entity, and [`PdsQuery`] to resolve it. A handle is scoped
//! to one repo of its account ([`RepoScope`]), the public one or an author's
//! within a space, a package's being `Pds::space`, and every caller (the
//! converge, the compiler) works on a scope without knowing which. Three
//! providers:
//!
//! - [`EmulatorPds`], a repo laid out in any [`BlobStore`] that performs a
//!   PDS's duties on write, for tests and for a repo that never leaves the
//!   machine, its records stored in canonical DAG-CBOR by default. Compiled
//!   under `dag_cbor` alone: the data model needs no network.
//! - `XrpcPds`, a real PDS over xrpc through a `RequestAuth` credential
//!   seam, with did and handle resolution, behind the `atproto` feature,
//!   which adds `json` for the wire.
//! - [`DryRunPds`], laid over either, keeping every write in memory: a dry
//!   run is the real run against one ([`Pds::dry`]), so its later steps read
//!   what its earlier ones would have written.
//!
//! A record body crosses as an `AtprotoValue`, sealed in the data model, which
//! is how a float reaches a repo that has none, and a record type is its body
//! alone: the rkey it lives at travels beside it, in the uri a read answers
//! and the `Rkeyed` a write takes. Every encoding goes through
//! `MediaType::serialize`, so `MediaType::DagCbor` writes exactly the block a
//! PDS stores and `AtprotoValue::cid` hashes. The `app.bsky.*` lexicons beet
//! reads and writes (`FeedPost`, `RichText`, the AppView's views) are the
//! `bsky` module's, plain data like the rest; reading them from a Bluesky
//! AppView (`AppView`) is behind `atproto`.
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
//! A test that writes to a real PDS writes to the test org,
//! `beetmania.beet.org`, the account the `beetmania` example publishes from
//! (its README's "Test accounts" lists it beside the test user). A live test
//! only ever writes records it deletes again, under its own collection or
//! publication url, so the org's own publication is never touched. Its app
//! password is sealed into the `agents` group of the checkout's
//! `secrets.toml`, which the test runner loads into the process environment:
//!
//! ```sh
//! beet secrets/set BEETMANIA_BEET_ORG_APP_PASSWORD --group=agents --role=env_var --note="app password for beetmania.beet.org, the test org" --roll="manual:https://bsky.app/settings/app-passwords"
//! ```
//!
//! A live write test reads it through `XrpcPds::test_org`, which answers
//! `None` with a note naming the secret when it is absent, so a fresh
//! checkout passes without it.
mod atproto_account;
mod atproto_plugin;
mod bsky;
mod converge;
mod dry_run_pds;
mod emulator_pds;
mod pds;
mod pds_query;
pub use atproto_account::*;
pub use atproto_plugin::*;
pub use bsky::*;
pub use converge::*;
pub use dry_run_pds::*;
pub use emulator_pds::*;
pub use pds::*;
pub use pds_query::*;
#[cfg(feature = "atproto")]
mod atproto_auth;
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
pub use identity::*;
#[cfg(feature = "atproto")]
pub use xrpc_pds::*;
