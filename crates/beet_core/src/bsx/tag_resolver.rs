//! The BSX custom-tag seam: an uppercase tag resolved by a registered handler
//! rather than the type registry or the [`BsxTemplateRegistry`].
//!
//! Core knows no concrete tag: like the [`EventRegistry`](crate::prelude::EventRegistry)
//! and [`VerbRegistry`](crate::prelude::VerbRegistry), this registry is empty by
//! default and populated by a downstream layer (eg `beet_ui` registers `Rule`).
//! A handler reads the parsed element's raw attributes and mutates the world,
//! producing no entity content — the markup analogue of a build-time effect like
//! a resource patch.
//!
//! The seam exists because some tags declare data whose home (eg a `RuleSet`)
//! lives in a higher crate that core cannot reference. A handler closes that gap
//! without core depending on the concrete type.

use super::ast::*;
use crate::prelude::*;
use alloc::sync::Arc;

/// A custom-tag handler: reads the parsed [`BsxElement`]'s raw attributes and
/// mutates the build entity (and through it the world via
/// [`EntityWorldMut::world_scope`]), producing no entity content.
pub type TagResolverFn =
	Arc<dyn Fn(&BsxElement, &mut EntityWorldMut) -> Result<()> + Send + Sync>;

/// Maps an uppercase tag name to its [`TagResolverFn`].
///
/// Consulted by the BSX resolver before the type registry, so a registered tag
/// (eg `Rule`) is handled here rather than resolved as a component/template.
/// Empty by default: core registers no tags.
#[derive(Default, Resource)]
pub struct BsxTagResolvers(HashMap<SmolStr, TagResolverFn>);

impl BsxTagResolvers {
	/// Register a handler for `tag`, replacing any existing one.
	pub fn insert(
		&mut self,
		tag: impl Into<SmolStr>,
		handler: impl Fn(&BsxElement, &mut EntityWorldMut) -> Result<()>
		+ Send
		+ Sync
		+ 'static,
	) -> &mut Self {
		self.0.insert(tag.into(), Arc::new(handler));
		self
	}

	/// The handler registered for `tag`, if any.
	pub fn get(&self, tag: &str) -> Option<TagResolverFn> {
		self.0.get(tag).cloned()
	}

	/// The registered handler-tag names, the source for a tooling catalog of every
	/// uppercase tag a BSX author can write that resolves to a build-time effect (eg
	/// `Rule`).
	pub fn keys(&self) -> impl Iterator<Item = &SmolStr> { self.0.keys() }
}
