#![allow(unused)]
/// this example is for macro expansion
use beet_rsx::as_beet::*;

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
impl StructD {
	fn to_self(self) -> Self { self }
}


#[derive(Debug, Default, Node)]
struct ButtonB {
	#[field(flatten)]
	#[field(flatten = "StructB<u32>")]
	#[field(flatten = "StructA<usize>")]
	#[field(flatten = StructC)]
	pub button_attrs: StructD,
	pub field_button_b: u32,
}
fn button_b(button: ButtonB) -> WebNode { WebNode::default() }



fn main() {
	let val = StructA::<u32>::default().field_a(2);
	assert_eq!(*val.get_field_a(), 2);
	let val = ButtonBBuilder::default()
		.field_a(2)
		.field_b(3)
		.field_c(true)
		.field_d(4)
		.field_button_b(5);
	assert_eq!(*val.get_field_a(), 2);
	assert_eq!(*val.get_field_b(), 3);
	assert_eq!(*val.get_field_c(), true);
	assert_eq!(*val.get_field_d(), 4);
	// assert_eq!(*val.get_field_button_b(), 5);
	println!("yay");
}
