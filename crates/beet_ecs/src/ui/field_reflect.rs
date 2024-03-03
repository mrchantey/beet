use std::fmt::Display;
use std::rc::Rc;


pub trait FieldValue: Clone {}
impl<T: Clone> FieldValue for T {}
pub type GetFunc<T> = Rc<Box<dyn Fn() -> T>>;
pub type SetFunc<T> = Rc<Box<dyn Fn(T)>>;

// TODO we should consider higher order functions for set, ie update_cb
// It may mean field reflect needs to know parent type
#[derive(Clone)]
pub struct FieldReflect<T: FieldValue> {
	pub field_name: String,
	pub display_name: String,
	pub get_cb: GetFunc<T>,
	pub set_cb: SetFunc<T>,
}

impl<T: FieldValue> Default for FieldReflect<T> {
	fn default() -> Self {
		Self::new(
			"PLACEHOLDER REFLECT VALUE".to_string(),
			|| {
				panic!("Placeholder value should not be used");
			},
			|_| {
				panic!("Placeholder value should not be used");
			},
		)
	}
}

impl<T: FieldValue> FieldReflect<T> {
	pub fn new(
		field_name: String,
		get_cb: impl 'static + Fn() -> T,
		set_cb: impl 'static + Fn(T),
	) -> Self {
		Self {
			display_name: heck::AsTitleCase(&field_name).to_string(),
			field_name,
			get_cb: Rc::new(Box::new(get_cb)),
			set_cb: Rc::new(Box::new(set_cb)),
		}
	}

	pub fn clone_get_cb(&self) -> GetFunc<T> { self.get_cb.clone() }
	pub fn clone_set_cb(&self) -> SetFunc<T> { self.set_cb.clone() }

	pub fn get(&self) -> T { (self.get_cb)() }
	pub fn set(&self, value: T) { (self.set_cb)(value) }
}

impl<T: FieldValue + Display> Display for FieldReflect<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.display_name, self.get())
	}
}
