//! DOM node types for beet applications.
//!
//! This module contains the core types for representing HTML DOM structures
//! in the Bevy ECS, including elements, attributes, text nodes, and templates.

mod escape_html;
mod event_observer;
mod signal_effect;
pub use event_observer::*;
pub use signal_effect::*;
/// Types for snippet root nodes and their components.
pub mod snippet_root;
pub use snippet_root::*;
/// Expression index tracking for template nodes.
pub mod expr_idx;
/// Template node types and traits.
pub mod template;
pub use expr_idx::*;
pub use template::*;
mod into_bundle;
pub use into_bundle::*;
/// RSX node types (fragments, templates, text, etc.).
pub mod rsx_nodes;
pub use escape_html::*;
pub use rsx_nodes::*;
mod web_nodes;
pub use web_nodes::*;
mod attribute;
pub use attribute::*;
mod directives;
pub use directives::*;
mod dom_idx;
pub use dom_idx::*;
mod html_constants;
pub use html_constants::*;

/// Added to the [`BuildPlugin`] and the [`ApplyDirectives`] for static scene
/// serde.
/// This plugin is not unique, so can be added in multiple places.
#[derive(Default)]
pub struct NodeTypesPlugin;

use beet_core::prelude::*;

/// A tuple of all RSX component type groups for registration.
///
/// This includes root components, RSX nodes, web nodes, and all directive types.
pub type RsxComponents = (
	RootComponents,
	RsxNodes,
	WebNodes,
	RsxDirectives,
	WebDirectives,
	LangDirectives,
);

impl bevy::app::Plugin for NodeTypesPlugin {
	fn is_unique(&self) -> bool { false }
	fn build(&self, app: &mut bevy::prelude::App) {
		app
			// bevy 0.17 doesnt register parent/child by default
			.register_type::<Children>()
			.register_type::<ChildOf>()
			// a blanket tuple, recursively registers all
			.register_type::<RsxComponents>()
			.register_type::<AttributeOf>()
			.register_type::<Attributes>();
	}
}
