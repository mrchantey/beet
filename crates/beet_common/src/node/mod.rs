mod event_observer;
pub mod rusty_tracker;
pub use event_observer::*;
pub use rusty_tracker::*;
mod into_template_bundle;
pub use into_template_bundle::*;
#[cfg(feature = "tokens")]
mod attribute_tokens;
#[cfg(feature = "tokens")]
pub use attribute_tokens::*;
pub mod rsx_nodes;
pub use rsx_nodes::*;
mod web_nodes;
pub use web_nodes::*;
mod item_of;
pub use item_of::*;
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



pub fn node_types_plugin(app: &mut bevy::prelude::App) {
	app
		// attributes
		.register_type::<AttributeOf>()
		.register_type::<Attributes>()
		.register_type::<AttributeKey>()
		.register_type::<AttributeLit>()
		// misc
		// .register_type::<OnClick>()
		.register_type::<ItemOf<TemplateNode, FileSpan>>()
		.register_type::<ItemOf<FragmentNode, FileSpan>>()
		.register_type::<ItemOf<TextNode, FileSpan>>()
		.register_type::<ItemOf<BlockNode, FileSpan>>()
		.register_type::<ItemOf<DoctypeNode, FileSpan>>()
		.register_type::<ItemOf<CommentNode, FileSpan>>()
		.register_type::<ItemOf<ElementNode, FileSpan>>()
		// rsx nodes
		.register_type::<NodeTag>()
		.register_type::<TemplateNode>()
		.register_type::<FragmentNode>()
		.register_type::<TextNode>()
		.register_type::<BlockNode>()
		// web nodes
		.register_type::<DoctypeNode>()
		.register_type::<CommentNode>()
		.register_type::<ElementNode>();
}
