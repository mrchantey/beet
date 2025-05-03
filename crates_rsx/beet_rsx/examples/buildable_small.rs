#![allow(unused)]
/// this example is for macro expansion
use beet_rsx::as_beet::*;

#[derive(Default, Buildable)]
pub struct StructA {
	pub field_a: usize,
}


#[derive(Default, Buildable)]
pub struct StructB {
	#[field(flatten)]
	pub struct_a: StructA,
	pub field_b: usize,
}
#[derive(Default, Buildable)]
pub struct StructC {
	#[field(flatten=StructA)]
	pub struct_b: StructB,
	pub field_c: usize,
}

fn main() {
	let val = StructC::default().field_a(1).field_b(2);
	assert_eq!(val.struct_b.struct_a.field_a, 1);
	assert_eq!(val.struct_b.field_b, 2);
	println!("you did it!");
}
