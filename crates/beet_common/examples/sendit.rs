use beet_common::as_beet::*;

fn main() {
	let foo = ElementNodeSend::new(ElementNode { self_closing: true });
	let _bar: ElementNodeSend = foo.clone();
}
#[derive(Sendit, Clone)]
#[sendit(derive(Clone))]
pub struct ElementNode {
	pub self_closing: bool,
}
