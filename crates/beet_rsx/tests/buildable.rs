#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_rsx::prelude::*;
use sweet::prelude::*;

#[test]
fn works() {
	let val = StructA::<u32>::default().field_a(2);
	val.get_field_a().xpect_eq(2);
	let val = ButtonB::default()
		.field_a(2)
		.field_b(3)
		.field_c(true)
		.field_d(4)
		.field_button_b(5);
	val.get_field_a().xpect_eq(2);
	val.get_field_b().xpect_eq(3);
	val.get_field_c().xpect_eq(true);
	val.get_field_d().xpect_eq(4);
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
	#[field(flatten = StructA<usize>)]
	pub struct_b: StructB<u32>,
	pub field_c: bool,
}
#[derive(Debug, Default, Buildable)]
pub struct StructD {
	#[field(flatten)]
	#[field(flatten = StructB<u32>)]
	#[field(flatten = StructA<usize>)]
	pub struct_c: StructC,
	pub field_d: u32,
}


#[derive(Debug, Default, Buildable)]
struct ButtonB {
	#[field(flatten)]
	#[field(flatten = StructB<u32>)]
	#[field(flatten = StructA<usize>)]
	#[field(flatten = StructC)]
	pub button_attrs: StructD,
	pub field_button_b: u32,
}
