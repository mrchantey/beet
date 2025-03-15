use beet_rsx::prelude::*;




#[derive(Node)]
// #[node(into_rsx)]
struct MyNode {
	is_required: u32,
	is_optional: Option<u32>,
	#[field(default = 7)]
	is_default: u32,
}

fn into_rsx(node: MyNode) -> RsxRoot { RsxRoot::default() }



fn main() {}
