mod event_observer;
pub use event_observer::*;
pub mod snippet_root;
pub use snippet_root::*;
pub mod expr_idx;
pub mod template;
pub use expr_idx::*;
pub use template::*;
mod into_template_bundle;
pub use into_template_bundle::*;
mod workspace_config;
pub use workspace_config::*;
pub mod rsx_nodes;
pub use rsx_nodes::*;
mod web_nodes;
pub use web_nodes::*;
mod attribute;
pub use attribute::*;
mod line_col;
pub use line_col::*;
mod file_span;
pub use file_span::*;
mod directives;
pub use directives::*;
mod dom_idx;
pub use dom_idx::*;
mod html_constants;
pub use html_constants::*;

/// Added to the [`BuildPlugin`] and the [`ApplyDirectives`] for static scene
/// serde.
/// This plugin is not unique, so can be added in multiple places.
pub struct NodeTypesPlugin;

// a group of all groups of components
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
			// a blanket tuple, recursively registers all
			.register_type::<RsxComponents>()
			.register_type::<AttributeOf>()
			.register_type::<Attributes>();
	}
}
