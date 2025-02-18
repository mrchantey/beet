mod slots_visitor;
mod tree_location;
mod tree_location_map;
pub use slots_visitor::*;
pub use tree_location::*;
pub use tree_location_map::*;
mod effect;
mod rsx_diff;
mod rsx_idx_incr;
mod rsx_location;
mod rsx_root_map;
mod rsx_template_map;
mod rsx_template_node;
mod rsx_template_root;
mod rsx_visitor_fn;
mod rusty_part;
#[cfg(feature = "css")]
mod scoped_style;
pub use rsx_idx_incr::*;
pub use rsx_root_map::*;
pub use rsx_template_map::*;
pub use rsx_template_node::*;
pub use rsx_template_root::*;
pub use rsx_visitor_fn::*;
pub use rusty_part::*;
#[cfg(feature = "css")]
pub use scoped_style::*;
mod rsx_node;
mod rsx_visitor;
pub use rsx_node::*;
pub use rsx_visitor::*;
pub use text_block_encoder::*;
mod rsx_root;
mod text_block_encoder;
pub use effect::*;
pub use rsx_location::*;
pub use rsx_root::*;


/// Unique identifier for every node in an rsx tree,
/// and assigned to html elements that need it.
/// The value is incremented every time an rsx node is encountered
/// in a dfs pattern like [RsxVisitor].
pub type RsxIdx = u32;


pub trait Rsx {
	fn into_rsx(self) -> RsxNode;
}

impl Rsx for RsxNode {
	fn into_rsx(self) -> RsxNode { self }
}
impl Rsx for RsxRoot {
	fn into_rsx(self) -> RsxNode { self.node }
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


pub trait IntoRsx<M> {
	fn into_rsx(self) -> RsxNode;
}

pub struct ToStringIntoRsx;
impl<T: ToString> IntoRsx<(T, ToStringIntoRsx)> for T {
	fn into_rsx(self) -> RsxNode { RsxNode::Text(self.to_string()) }
}
pub struct FuncIntoRsx;
impl<T: FnOnce() -> U, U: IntoRsx<M2>, M2> IntoRsx<(M2, FuncIntoRsx)> for T {
	fn into_rsx(self) -> RsxNode { self().into_rsx() }
}


pub trait Component {
	fn render(self) -> RsxRoot;
}

impl<T: FnOnce() -> RsxRoot> Component for T {
	fn render(self) -> RsxRoot { self() }
}

impl<T: Component> Rsx for T {
	fn into_rsx(self) -> RsxNode { self.render().into_rsx() }
}
