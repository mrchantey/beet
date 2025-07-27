use beet_core::as_beet::*;


fn main() {}
#[derive(ToTokens)]
#[to_tokens(Self::new)]
pub struct ElementNode {
	pub self_closing: bool,
}

impl ElementNode {
	pub fn new(self_closing: bool) -> Self {
		Self { self_closing }
	}
}