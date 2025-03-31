mod effect;
mod global_rsx_idx;
mod pipeline;
mod props;
mod rsx_location;
mod rsx_node;
pub use rsx_visitor_fn::*;
mod rsx_visitor;
mod rsx_visitor_fn;
mod tree_idx;
pub use effect::*;
pub use global_rsx_idx::*;
pub use pipeline::*;
pub use props::*;
pub use rsx_location::*;
pub use rsx_node::*;
pub use rsx_visitor::*;
pub use tree_idx::*;

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
