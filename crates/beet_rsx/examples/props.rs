// probs should be a test but so nice for cargo expand

#![feature(more_qualified_paths)]
use beet_rsx::as_beet::*;
use std::marker::PhantomData;


fn some_custom_name(props: MyNode) -> RsxNode {
	rsx! {
		<div>
			<p>is_optional: {format!("{:?}", props.is_optional)}</p>
			<p>is_required: {format!("{:?}", props.is_required)}</p>
			<p>is_default: {format!("{:?}", props.is_default)}</p>
			<p>is_generic_default: {format!("{:?}", props.is_generic_default)}</p>
			<p>is_into: {format!("{:?}", props.is_no_into)}</p>
			<p>is_boxed: {format!("{:?}", (props.is_boxed)())}</p>
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
			is_boxed=|| 3
			is_no_into="foobar".into()
			is_optional=3
			class="kablamo"
			id="bar"
		/>
	}
	.xpipe(RsxToHtmlString::default())
	.unwrap();
	assert_eq!(
		str,
		"<div><p data-beet-rsx-idx=\"4\">is_optional: Some(3)</p><p data-beet-rsx-idx=\"9\">is_required: 38</p><p data-beet-rsx-idx=\"14\">is_default: 7</p><p data-beet-rsx-idx=\"19\">is_generic_default: Foo(PhantomData<u32>)</p><p data-beet-rsx-idx=\"24\">is_into: \"foobar\"</p><p data-beet-rsx-idx=\"29\">is_boxed: 3</p><p class=\"kablamo\" id=\"bar\" data-beet-rsx-idx=\"34\">is_flatten: MyFlattenedNode { class: \"kablamo\", id: Some(\"bar\"), disabled: None }</p></div>"
	);
	sweet::log!("success!");
}


#[derive(Debug, Default)]
struct Foo<T>(PhantomData<T>);

#[derive(Debug, Default, Clone, Buildable, IntoRsxAttributes)]
struct MyFlattenedNode {
	/// the class that will be set
	class: String,
	/// this is what identifies it
	id: Option<String>,
	disabled: Option<bool>,
}

#[derive(Node)]
#[node(into_rsx=some_custom_name)]
struct MyNode {
	/// This is a comment
	is_required: u32,
	is_boxed: Box<dyn Fn() -> u32>,
	is_optional: Option<u32>,
	#[field(default = 7)]
	is_default: u32,
	#[field(default)]
	is_generic_default: Foo<u32>,
	#[field(no_into)]
	is_no_into: String,
	#[field(flatten)]
	is_flatten: MyFlattenedNode,
	// #[field(foo)]
	// is_bad_macro: String,
}
