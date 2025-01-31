mod rsx_context;
mod rsx_node;
pub use rsx_context::*;
pub use rsx_node::*;
pub use text_block_encoder::*;
mod text_block_encoder;

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
	fn render(self) -> impl Rsx;
}

impl<T: FnOnce() -> RsxNode> Component for T {
	fn render(self) -> impl Rsx { self() }
}

impl<T: Component> Rsx for T {
	fn into_rsx(self) -> RsxNode { self.render().into_rsx() }
}
