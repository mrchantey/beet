//! A store the deploy names but does not create.

use crate::prelude::*;
use beet_core::prelude::*;

/// `<StoreUriBlock label="repo" uri="s3://my-bucket" {RepoStoreBlock}/>` — a
/// [`StoreBlock`] that is nothing but a uri: a store provided by something
/// other than this deploy (a bucket another stack or an operator owns, a
/// directory on the box, browser storage), declared so the consumers reading a
/// store through its erased half (the repo store a compute boots from, the
/// ledger) reach it exactly as they reach a bucket the deploy creates.
///
/// Emits no resource and declares no grant, since what it names is not this
/// deploy's to provision or permit; a process reading it brings its own access.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedStoreBlock::on_insert::<Self>,
	on_remove = ErasedStoreBlock::on_remove
)]
pub struct StoreUriBlock {
	label: SmolStr,
	/// The store as a process is told to read it, ie `s3://<bucket>` or
	/// `fs:/srv/site`, in the spelling `--repo` parses.
	uri: StoreUri,
	/// Nest each deploy's content under its id below the uri, see
	/// [`StoreBlock::deploy_versioned`]. Off by default: a store this deploy
	/// does not own is not one it versions unless told to.
	deploy_versioned: bool,
}

impl Default for StoreUriBlock {
	fn default() -> Self { Self::new("", StoreUri::default()) }
}

impl StoreUriBlock {
	pub fn new(label: impl Into<SmolStr>, uri: StoreUri) -> Self {
		Self {
			label: label.into(),
			uri,
			deploy_versioned: false,
		}
	}
}

impl Block for StoreUriBlock {
	fn label(&self) -> &SmolStr { &self.label }
}

impl StoreBlock for StoreUriBlock {
	fn store_uri(&self, _stack: &ResolvedStack) -> StoreUri { self.uri.clone() }

	fn deploy_versioned(&self) -> bool { self.deploy_versioned }
}
