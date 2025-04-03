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
			<p>is_into: {format!("{:?}", props.is_no_into)}</p>
			<p {props
				.is_flatten
				.clone()}>is_flatten: {format!("{:?}", props.is_flatten)}</p>
		</div>
	}
}



fn main() {
	let str = rsx! {
		<MyNode
			is_required=38
			is_no_into="foobar".into()
			is_optional=3
			class="kablamo"
		/>
	}
	.bpipe(RsxToHtmlString::default())
	.unwrap();
	assert_eq!(
		str,
		"<div><p data-beet-rsx-idx=\"4\">is_optional: Some(3)</p><p data-beet-rsx-idx=\"9\">is_required: 38</p><p data-beet-rsx-idx=\"14\">is_default: 7</p><p data-beet-rsx-idx=\"19\">is_generic_default: Foo(PhantomData<u32>)</p><p data-beet-rsx-idx=\"24\">is_into: \"foobar\"</p><p class=\"kablamo\" data-beet-rsx-idx=\"29\">is_flatten: SomeHtmlAttrs { class: \"kablamo\" }</p></div>"
	);
	sweet::log!("success!");
}


#[derive(Debug, Default)]
struct Foo<T>(PhantomData<T>);


#[derive(Node)]
#[node(into_rsx=my_node_other)]
struct MyNode {
	is_required: u32,
	is_optional: Option<u32>,
	#[field(default = 7)]
	is_default: u32,
	#[field(default)]
	is_generic_default: Foo<u32>,
	#[field(no_into)]
	is_no_into: String,
	#[field(flatten)]
	is_flatten: SomeHtmlAttrs,
	// #[field(foo)]
	// is_bad_macro: String,
}


#[derive(Debug, Default, Clone, Buildable, IntoRsxAttributes)]
struct SomeHtmlAttrs {
	class: String,
}
