use std::marker::PhantomData;

use beet_rsx::prelude::*;

#[derive(Default)]
struct Foo<T>(PhantomData<T>);

#[derive(Node)]
#[node(into_rsx=foo)]
struct MyNode {
	is_required: u32,
	is_optional: Option<u32>,
	#[field(default = 7)]
	is_default: u32,
	#[field(default)]
	another_default: Foo<u32>,
}

fn foo(node: MyNode) -> RsxRoot { RsxRoot::default() }



fn main() {}
