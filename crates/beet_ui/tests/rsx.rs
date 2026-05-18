
beet_core::test_main!();

use beet_core::prelude::*;
// resolve crate:: aliasing in macros
use beet_ui::*;

/// Verify the macro compiles by checking bundle validity.
fn is_bundle(_: impl Bundle) {}

#[beet_core::test]
fn single_element() {
	is_bundle(rsx! {
		<div/>
	});
}

#[beet_core::test]
fn element_with_flag_attribute() {
	is_bundle(rsx! {
		<div my_flag/>
	});
}

#[beet_core::test]
fn element_with_key_value_attribute() {
	let bar = 2;
	is_bundle(rsx! {
		<div foo=bar/>
	});
}

#[beet_core::test]
fn element_with_string_attribute() {
	is_bundle(rsx! {
		<div bazz="boo"/>
	});
}

#[beet_core::test]
fn element_with_literal_attribute() {
	is_bundle(rsx! {
		<div bang=3/>
	});
}

#[beet_core::test]
fn element_with_block_spread() {
	let foo = Name::new("foo");
	let boo = Name::new("boo");
	is_bundle(rsx! {
		<div {(foo,boo)}/>
	});
}

#[beet_core::test]
fn element_with_children() {
	is_bundle(rsx! {
		<div>"hello"</div>
	});
}

#[beet_core::test]
fn multiple_root_elements() {
	is_bundle(rsx! {
		<div/>
		<div/>
	});
}

#[beet_core::test]
fn nested_elements() {
	is_bundle(rsx! {
		<div>
			<span>"inner"</span>
		</div>
	});
}

#[derive(Debug, Default, Component, SetWith)]
#[allow(dead_code)]
struct MyComponent {
	foo: bool,
	bar: String,
	bazz: u32,
}

#[beet_core::test]
fn component_with_set_with() {
	is_bundle(rsx! {
		<MyComponent foo bar="hello"/>
	});
}

#[beet_core::test]
fn mixed_elements_and_components() {
	is_bundle(rsx! {
		<div>
			<MyComponent foo/>
			<span>"text"</span>
		</div>
	});
}

#[beet_core::test]
fn combined_attributes() {
	let bar = 2;
	let foo = Name::new("test");
	let boo = Name::new("test2");
	is_bundle(rsx! {
		<div my_flag foo=bar bazz="boo" bang=3 {(foo,boo)}>"child"</div>
	});
}


#[beet_core::test]
fn component_with_block_attr_inserts_additional_component() {
	let extra = Name::new("extra");
	is_bundle(rsx! {
		<MyComponent foo {extra}/>
	});
}

#[beet_core::test]
fn component_with_children() {
	is_bundle(rsx! {
		<MyComponent foo>
			<span>"child"</span>
		</MyComponent>
	});
}

#[beet_core::test]
fn doctype_node() {
	is_bundle(rsx! {
		<!DOCTYPE html>
	});
}
