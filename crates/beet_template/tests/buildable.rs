#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_template::as_beet::*;
use bevy::prelude::*;
use sweet::prelude::*;

#[test]
fn works() {
	let val = StructA::<u32>::default().field_a(2);
	expect(*val.get_field_a()).to_be(2);
	let val = ButtonB::default()
		.field_a(2)
		.field_b(3)
		.field_c(true)
		.field_d(4)
		.field_button_b(5);
	expect(*val.get_field_a()).to_be(2);
	expect(*val.get_field_b()).to_be(3);
	expect(*val.get_field_c()).to_be(true);
	expect(*val.get_field_d()).to_be(4);
}


#[derive(Debug, Default, Buildable)]
pub struct StructA<T: 'static + Clone>
where
	T: ToString,
{
	pub field_a: T,
}

#[derive(Debug, Default, Buildable)]
pub struct StructB<T: 'static> {
	#[field(flatten)]
	pub struct_a: StructA<usize>,
	pub field_b: T,
}
#[derive(Debug, Default, Buildable)]
pub struct StructC {
	#[field(flatten)]
	#[field(flatten = "StructA<usize>")]
	pub struct_b: StructB<u32>,
	pub field_c: bool,
}
#[derive(Debug, Default, Buildable)]
pub struct StructD {
	#[field(flatten)]
	#[field(flatten = "StructB<u32>")]
	#[field(flatten = "StructA<usize>")]
	pub struct_c: StructC,
	pub field_d: u32,
}


#[derive(Debug, Default, Buildable)]
struct ButtonB {
	#[field(flatten)]
	#[field(flatten = "StructB<u32>")]
	#[field(flatten = "StructA<usize>")]
	#[field(flatten = StructC)]
	pub button_attrs: StructD,
	pub field_button_b: u32,
}
