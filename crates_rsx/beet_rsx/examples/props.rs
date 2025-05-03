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
			<p>is_flatten.class: {format!("{:?}", props.is_flatten.class)}</p>
			<p>is_flatten.id: {format!("{:?}", props.is_flatten.id)}</p>
			<p>is_flatten.disabled: {format!("{:?}", props.is_flatten.disabled)}</p>
			<p>is_marker_into: {format!("{:?}", props.is_marker_into)}</p>
			<p>is_maybe_signal: {format!("{:?}", props.is_maybe_signal)}</p>
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
			is_marker_into=3
			is_maybe_signal=|| 5u32
			class="kablamo"
			id="bar"
		/>
	}
	.xpipe(RsxToHtmlString::default())
	.unwrap();
	assert_eq!(
		str,
		"<div><p data-beet-rsx-idx=\"4\">is_optional: Some(3)</p><p data-beet-rsx-idx=\"9\">is_required: 38</p><p data-beet-rsx-idx=\"14\">is_default: 7</p><p data-beet-rsx-idx=\"19\">is_generic_default: Foo(PhantomData<u32>)</p><p data-beet-rsx-idx=\"24\">is_into: \"foobar\"</p><p data-beet-rsx-idx=\"29\">is_boxed: 3</p><p data-beet-rsx-idx=\"34\">is_flatten.class: \"kablamo\"</p><p data-beet-rsx-idx=\"39\">is_flatten.id: Some(\"bar\")</p><p data-beet-rsx-idx=\"44\">is_flatten.disabled: None</p><p data-beet-rsx-idx=\"49\">is_marker_into: \"3\"</p><p data-beet-rsx-idx=\"54\">is_maybe_signal: Func(5)</p></div>"
	);
	sweet::log!("success!");
}

trait MarkerIntoString<M> {
	fn marker_into_string(self) -> String;
}
impl MarkerIntoString<()> for u32 {
	fn marker_into_string(self) -> String { self.to_string() }
}


#[derive(Debug, Default)]
struct Foo<T>(PhantomData<T>);

#[derive(Default, Buildable, IntoBlockAttribute)]
struct MyFlattenedNode {
	/// the class that will be set
	class: String,
	/// this is what identifies it
	id: Option<String>,
	disabled: Option<bool>,
	onclick: Option<Box<dyn EventHandler<MouseEvent>>>,
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

	#[field(into_type = "impl MarkerIntoString<M>")]
	#[field(into_generics = "<M>")]
	#[field(into_func=marker_into_string)]
	is_marker_into: String,

	is_maybe_signal: MaybeSignal<u32>,
	// #[field(foo)]
	// is_bad_macro: String,
}
