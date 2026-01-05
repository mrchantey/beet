#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;
use beet_rsx::prelude::*;
use std::marker::PhantomData;

#[test]
fn works() {
	let (get, _set) = signal(7);
	rsx! {
		<MyNode
			is_required=38
			is_boxed=|| 3
			type="foo"
			is_no_into="foobar".into()
			is_optional=3
			is_marker_into=3
			is_derived_getter=get
			class="kablamo"
			onclick=|| println!("hello world")
			id="bar"
		/>
	}
	.xmap(HtmlFragment::parse_bundle)
	.xpect_snapshot();
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

#[derive(Default, Buildable, AttributeBlock)]
struct MyFlattenedNode {
	/// the class that will be set
	class: String,
	/// this is what identifies it
	id: Option<String>,
	disabled: Option<bool>,
	onclick: Option<EventHandler<OnClick>>,
}

#[template]
fn MyNode(
	/// This is a comment
	is_required: u32,
	is_boxed: Box<dyn Fn() -> u32 + Send + Sync>,
	is_optional: Option<u32>,
	r#type: String,
	#[field(default = 7)] is_default: u32,
	#[field(default)] is_generic_default: Foo<u32>,
	#[field(no_into)] is_no_into: String,
	#[field(flatten)] is_flatten: MyFlattenedNode,

	#[field(into_type = impl MarkerIntoString<M>)]
	#[field(into_generics = <M>)]
	#[field(into_func=marker_into_string)]
	is_marker_into: String,
	is_derived_getter: DerivedGetter<u32>,
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
			<p>type: {format!("{:?}", r#type)}</p>
			<p>is_boxed: {format!("{:?}", (is_boxed)())}</p>
			<p>is_flatten.class: {format!("{:?}", is_flatten.class)}</p>
			<p>is_flatten.id: {format!("{:?}", is_flatten.id)}</p>
			<p>is_flatten.disabled: {format!("{:?}", is_flatten.disabled)}</p>
			<p>is_marker_into: {format!("{:?}", is_marker_into)}</p>
			<p>is_derived_getter: {format!("{:?}", is_derived_getter)}</p>
		</div>
	}
}
