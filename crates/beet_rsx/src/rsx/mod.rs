mod apply_template_to_node;
mod effect;
mod global_rsx_idx;
mod node_to_template;
mod pipeline;
mod props;
mod rsx_diff;
mod rsx_location;
mod rsx_node;
mod rsx_template_map;
mod rsx_template_node;
mod rsx_visitor;
mod rsx_visitor_fn;
mod rusty_part;
mod text_block_encoder;
mod tree_idx;
mod tree_location;
mod tree_location_map;
pub use apply_template_to_node::*;
pub use effect::*;
pub use global_rsx_idx::*;
pub use node_to_template::*;
pub use pipeline::*;
pub use props::*;
pub use rsx_location::*;
pub use rsx_node::*;
pub use rsx_template_map::*;
pub use rsx_template_node::*;
pub use rsx_visitor::*;
pub use rsx_visitor_fn::*;
pub use rusty_part::*;
pub use text_block_encoder::*;
pub use tree_idx::*;
pub use tree_location::*;
pub use tree_location_map::*;

// TODO deprecate for IntoRsxNode
pub trait Rsx {
	fn into_rsx(self) -> RsxNode;
}

impl Rsx for RsxNode {
	fn into_rsx(self) -> RsxNode { self }
}
impl Rsx for () {
	fn into_rsx(self) -> RsxNode { RsxNode::default() }
}


// impl Rsx for &str {
// 	fn into_rsx(self) -> RsxNode { RsxNode::Text(self.to_string()) }
// }
// impl Rsx for String {
// 	fn into_rsx(self) -> RsxNode { RsxNode::Text(self) }
// }

pub trait Component {
	fn render(self) -> RsxNode;
}

impl<T: FnOnce() -> RsxNode> Component for T {
	fn render(self) -> RsxNode { self() }
}

impl<T: Component> Rsx for T {
	fn into_rsx(self) -> RsxNode { self.render().into_rsx() }
}
