mod effect;
mod global_rsx_idx;
mod pipeline;
mod props;
mod rsx_location;
mod rsx_node;
mod rsx_node_meta;
mod rsx_visitor;
mod rsx_visitor_fn;
mod template_directive;
mod tree_idx;
pub use effect::*;
pub use global_rsx_idx::*;
pub use pipeline::*;
pub use props::*;
pub use rsx_location::*;
pub use rsx_node::*;
pub use rsx_node_meta::*;
pub use rsx_visitor::*;
pub use rsx_visitor_fn::*;
pub use template_directive::*;
pub use tree_idx::*;


// probs deprecate for IntoRsx which has a default T
pub trait Component {
	fn render(self) -> RsxNode;
}

impl<T: FnOnce() -> RsxNode> Component for T {
	fn render(self) -> RsxNode { self() }
}
