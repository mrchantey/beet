mod effect;
mod rsx_hydrated_node;
mod rsx_location;
mod rsx_template_node;
mod rsx_visitor_fn;
mod scoped_style;
pub use rsx_hydrated_node::*;
pub use rsx_template_node::*;
pub use rsx_visitor_fn::*;
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

pub trait IntoRsxAttributeValue<M> {
	fn into_attribute_value(self) -> String;
}

pub struct ToStringIntoRsxAttributeValue;
impl<T: ToString> IntoRsxAttributeValue<(T, ToStringIntoRsxAttributeValue)>
	for T
{
	fn into_attribute_value(self) -> String { self.to_string() }
}
pub struct FuncIntoRsxAttribute;
impl<T: FnOnce() -> U, U: IntoRsxAttributeValue<M2>, M2>
	IntoRsxAttributeValue<(M2, FuncIntoRsxAttribute)> for T
{
	fn into_attribute_value(self) -> String { self().into_attribute_value() }
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
