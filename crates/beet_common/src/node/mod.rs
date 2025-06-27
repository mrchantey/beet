mod event_observer;
mod on_spawn_template;
pub use event_observer::*;
pub use on_spawn_template::*;
pub mod macro_idx;
pub use macro_idx::*;
pub mod expr_idx;
pub use expr_idx::*;
mod into_template_bundle;
pub use into_template_bundle::*;
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
mod html_constants;
pub use html_constants::*;


/// Added to the [`StaticScenePlugin`] and the [`TemplatePlugin`] for static scene
/// serde.
/// This plugin is not unique, so can be added in multiple places.
pub struct NodeTypesPlugin;

impl bevy::app::Plugin for NodeTypesPlugin {
	// allow to be added in several places
	fn is_unique(&self) -> bool { false }
	fn build(&self, app: &mut bevy::prelude::App) {
		app
			// idxs
			// we dont need DomIdx, idxs applied after serde
			// .register_type::<DomIdx>()
			.register_type::<MacroIdx>()
			.register_type::<ExprIdx>()
			// rsx nodes
			.register_type::<NodeTag>()
			.register_type::<TemplateNode>()
			.register_type::<FragmentNode>()
			.register_type::<TextNode>()
			.register_type::<BlockNode>()
			.register_type::<FileSpanOf<NodeTag>>()
			.register_type::<FileSpanOf<TemplateNode>>()
			.register_type::<FileSpanOf<FragmentNode>>()
			.register_type::<FileSpanOf<TextNode>>()
			.register_type::<FileSpanOf<BlockNode>>()
			// web nodes
			.register_type::<DoctypeNode>()
			.register_type::<CommentNode>()
			.register_type::<ElementNode>()
			.register_type::<FileSpanOf<DoctypeNode>>()
			.register_type::<FileSpanOf<CommentNode>>()
			.register_type::<FileSpanOf<ElementNode>>()
			// directives
			.register_type::<LangContent>()
			.register_type::<HtmlHoistDirective>()
			.register_type::<ClientLoadDirective>()
			.register_type::<ClientOnlyDirective>()
			.register_type::<SlotChild>()
			.register_type::<SlotTarget>()
			// attributes
			.register_type::<AttributeOf>()
			.register_type::<Attributes>()
			.register_type::<AttributeKey>()
			.register_type::<AttributeLit>()
			//_
			;
	}
}
