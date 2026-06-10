//! The template build lifecycle: events, the error component, and the
//! generalized pending-dependency set that gates [`LoadTemplate`].
//!
//! The lifecycle has two observable boundaries on a template root:
//!
//! - [`SpawnTemplate`] fires once after the root's subtree is built and its
//!   slots resolved. It is the "built" signal and the attach point for future
//!   subtree passes.
//! - [`LoadTemplate`] fires when the root's [`TemplatePending`] set drains,
//!   immediately after [`SpawnTemplate`] when nothing is pending. It carries
//!   [`LoadTemplate::is_error`] and fires whether the load succeeded or failed.
//!
//! Build, validation, and load failures never panic and are never returned as
//! an `Err` from `spawn_template`. They insert [`TemplateError`] on the root
//! and drive [`LoadTemplate`] with `is_error: true`.

use crate::prelude::*;

/// Fired once on a template root after its subtree is built and its slots are
/// resolved.
///
/// This is the post-build phase boundary: the observable hook a future subtree
/// pass attaches to without modifying the walker. For a single `spawn_template`
/// call it fires exactly once, on the root.
#[derive(Debug, Clone, EntityEvent)]
pub struct SpawnTemplate {
	/// The template root.
	pub entity: Entity,
}

/// Fired on a template root when its [`TemplatePending`] dependency set drains.
///
/// Fires whether the load succeeded or failed; [`Self::is_error`] is `true`
/// when the root carries a [`TemplateError`]. When nothing is pending it fires
/// synchronously, immediately after [`SpawnTemplate`].
#[derive(Debug, Clone, EntityEvent)]
pub struct LoadTemplate {
	/// The template root.
	pub entity: Entity,
	/// `true` when the root failed; read [`TemplateError`] off the root.
	pub is_error: bool,
}

/// Inserted on a template root whose build, validation, or load failed.
///
/// Build failures ride this path rather than panicking or returning an `Err`:
/// the walker inserts this component and fires [`LoadTemplate`] with
/// `is_error: true`.
#[derive(Debug, Component)]
pub struct TemplateError {
	/// The underlying error.
	pub error: BevyError,
}

impl TemplateError {
	/// Wraps an error for insertion on a failed root.
	pub fn new(error: impl Into<BevyError>) -> Self {
		Self {
			error: error.into(),
		}
	}
}

/// The set of outstanding dependencies gating [`LoadTemplate`] on a root.
///
/// Generalized so assets, schemas, and remote fetches register into it later.
/// Each dependency is an opaque [`PendingId`]. The set fires [`LoadTemplate`]
/// when it drains to empty (via [`drain_pending_dependencies`]).
///
/// For Task 0 there are no dependencies, so a root that registers none drains
/// immediately and [`LoadTemplate`] fires synchronously within `spawn_template`.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct TemplatePending {
	/// The outstanding dependency ids.
	ids: HashSet<PendingId>,
	/// The next id to hand out from [`Self::register`].
	next: u64,
}

/// An opaque identifier for one pending dependency on a [`TemplatePending`] set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct PendingId(u64);

/// The template root currently being built, set by the build walker for the
/// duration of a [`spawn_template`](crate::prelude::WorldTemplateExt) build.
///
/// A deferred dependency (an asset, a remote schema, a remote template) reads
/// this to know which entity carries the [`TemplatePending`] set its
/// [`PendingId`] must park on, so [`LoadTemplate`] defers until it resolves.
/// Absent outside a build, in which case a dependency registers on the entity it
/// builds into.
#[derive(Debug, Clone, Copy, Resource)]
pub struct TemplateBuildRoot(pub Entity);

impl TemplateBuildRoot {
	/// The build root recorded in `world`, falling back to `entity` when none is
	/// set (a build outside the walker), so a deferred dependency always has a
	/// root to park on.
	pub fn resolve(world: &World, entity: Entity) -> Entity {
		world
			.get_resource::<TemplateBuildRoot>()
			.map(|root| root.0)
			.unwrap_or(entity)
	}
}

impl TemplatePending {
	/// Registers a new dependency, returning its [`PendingId`].
	///
	/// While any dependency is registered, [`LoadTemplate`] is deferred until
	/// every one is resolved via [`Self::resolve`].
	pub fn register(&mut self) -> PendingId {
		let id = PendingId(self.next);
		self.next += 1;
		self.ids.insert(id);
		id
	}

	/// Resolves a previously registered dependency.
	///
	/// Returns `true` if the set is now empty, ie ready to fire [`LoadTemplate`].
	pub fn resolve(&mut self, id: PendingId) -> bool {
		self.ids.remove(&id);
		self.is_empty()
	}

	/// Returns `true` if no dependencies are outstanding.
	pub fn is_empty(&self) -> bool { self.ids.is_empty() }

	/// The number of outstanding dependencies.
	pub fn len(&self) -> usize { self.ids.len() }
}

/// Fires [`LoadTemplate`] on `root` if its [`TemplatePending`] set is empty (or
/// absent), reporting the error state from the presence of [`TemplateError`].
///
/// This is the drain trigger. It is called synchronously by the walker after
/// [`SpawnTemplate`] (the empty case), and is the same call a dependency
/// resolver makes once it observes the set has drained. Calling it while
/// dependencies remain is a no-op, so a resolver may call it unconditionally
/// after [`TemplatePending::resolve`] returns `true`.
pub fn drain_pending_dependencies(root: &mut EntityWorldMut) {
	let pending_empty = root
		.get::<TemplatePending>()
		.map(TemplatePending::is_empty)
		.unwrap_or(true);
	if !pending_empty {
		return;
	}
	let is_error = root.contains::<TemplateError>();
	root.trigger(move |entity| LoadTemplate { entity, is_error });
}
