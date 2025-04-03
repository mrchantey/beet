#![feature(more_qualified_paths)]
use beet_rsx::as_beet::*;
use std::marker::PhantomData;


fn my_node_other(props: MyNode) -> RsxNode {
	rsx! {
		<div>
			<p>is_optional: {format!("{:?}", props.is_optional)}</p>
			<p>is_required: {format!("{:?}", props.is_required)}</p>
			<p>is_default: {format!("{:?}", props.is_default)}</p>
			<p>is_generic_default: {format!("{:?}", props.is_generic_default)}</p>
			<p>is_into: {format!("{:?}", props.is_into)}</p>
		</div>
	}
}



fn main() {
	let str = rsx! { <MyNode
	is_required=38
	is_into="foobar"
	is_optional=3
	// foo="kablamo"
	/> }
	.bpipe(RsxToHtmlString::default())
	.unwrap();
	assert_eq!(
		str,
		"<div><p data-beet-rsx-idx=\"3\">is_optional: None</p><p data-beet-rsx-idx=\"8\">is_required: 38</p><p data-beet-rsx-idx=\"13\">is_default: 7</p><p data-beet-rsx-idx=\"18\">is_generic_default: Foo(PhantomData<u32>)</p><p data-beet-rsx-idx=\"23\">is_into: \"foobar\"</p></div>"
	);
	sweet::log!("success!");
}


#[derive(Debug, Default)]
struct Foo<T>(PhantomData<T>);

#[derive(Default)]
struct SomeOtherType {
	foo: String,
}

#[derive(Node)]
#[node(into_rsx=my_node_other)]
struct MyNode {
	is_required: u32,
	is_optional: Option<u32>,
	#[field(default = 7)]
	is_default: u32,
	#[field(default)]
	is_generic_default: Foo<u32>,
	#[field(into)]
	is_into: String,
	#[field(flatten)]
	is_flatten: SomeOtherType,
	// #[field(foo)]
	// is_bad_macro: String,
}
