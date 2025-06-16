// probs should be a test but so nice for cargo expand
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(more_qualified_paths)]
use beet_template::as_beet::*;
use std::marker::PhantomData;
use sweet::prelude::*;


#[test]
fn works() {
	let (get, _set) = signal(7);
	rsx! {
		<MyNode
			is_required=38
			is_boxed=|| 3
			is_no_into="foobar".into()
			is_optional=3
			is_marker_into=3
			is_maybe_signal=get
			class="kablamo"
			id="bar"
		/>
	}
	.xmap(HtmlFragment::parse_bundle)
	.xpect().to_be(
		"<div><p>is_optional: Some(3)</p><p>is_required: 38</p><p>is_default: 7</p><p>is_generic_default: Foo(PhantomData<u32>)</p><p>is_into: \"foobar\"</p><p>is_boxed: 3</p><p>is_flatten.class: \"kablamo\"</p><p>is_flatten.id: Some(\"bar\")</p><p>is_flatten.disabled: None</p><p>is_marker_into: \"3\"</p><p>is_maybe_signal: Getter(7)</p></div>"
	);
}

#[allow(unused)]
trait MarkerIntoString<M> {
	fn marker_into_string(self) -> String;
}
impl MarkerIntoString<()> for u32 {
	fn marker_into_string(self) -> String { self.to_string() }
}


#[derive(Debug, Default)]
struct Foo<T>(PhantomData<T>);

#[derive(Default, Buildable, TemplateBundle)]
struct MyFlattenedNode {
	/// the class that will be set
	class: String,
	/// this is what identifies it
	id: Option<String>,
	disabled: Option<bool>,
	onclick: Option<EntityObserver>,
}

#[template]
fn MyNode(
	/// This is a comment
	is_required: u32,
	is_boxed: Box<dyn Fn() -> u32 + Send + Sync>,
	is_optional: Option<u32>,
	#[field(default = 7)] is_default: u32,
	#[field(default)] is_generic_default: Foo<u32>,
	#[field(no_into)] is_no_into: String,
	#[field(flatten)] is_flatten: MyFlattenedNode,

	#[field(into_type = "impl MarkerIntoString<M>")]
	#[field(into_generics = "<M>")]
	#[field(into_func=marker_into_string)]
	is_marker_into: String,
	is_maybe_signal: MaybeSignal<u32>,
	// #[field(foo)]
	// is_bad_macro: String,
) -> impl Bundle {
	rsx! {
		<div>
			<p>is_optional: {format!("{:?}", is_optional)}</p>
			<p>is_required: {format!("{:?}", is_required)}</p>
			<p>is_default: {format!("{:?}", is_default)}</p>
			<p>is_generic_default: {format!("{:?}", is_generic_default)}</p>
			<p>is_into: {format!("{:?}", is_no_into)}</p>
			<p>is_boxed: {format!("{:?}", (is_boxed)())}</p>
			<p>is_flatten.class: {format!("{:?}", is_flatten.class)}</p>
			<p>is_flatten.id: {format!("{:?}", is_flatten.id)}</p>
			<p>is_flatten.disabled: {format!("{:?}", is_flatten.disabled)}</p>
			<p>is_marker_into: {format!("{:?}", is_marker_into)}</p>
			<p>is_maybe_signal: {format!("{:?}", is_maybe_signal)}</p>
		</div>
	}
}
