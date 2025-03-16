use once_cell::sync::Lazy;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::RwLock;

static CONTEXT: Lazy<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>> =
	Lazy::new(|| RwLock::new(HashMap::new()));


/// Get a global context value, cascading is not currently supported
pub fn get_context<T: 'static + Clone + Send + Sync>() -> T {
	try_get_context().expect("Context value not found")
}

/// Try to get a global context value, cascading is not currently supported
pub fn try_get_context<T: 'static + Clone + Send + Sync>() -> Option<T> {
	let cx_map = CONTEXT.read().unwrap();
	cx_map
		.get(&TypeId::of::<T>())
		.and_then(|value| value.downcast_ref::<T>())
		.cloned()
}
/// Set a global context value, cascading is not currently supported
pub fn set_context<T: 'static + Clone + Send + Sync>(value: T) {
	let mut cx_map = CONTEXT.write().unwrap();
	cx_map.insert(TypeId::of::<T>(), Box::new(value));
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		set_context(7);
		expect(get_context::<i32>()).to_be(7);

		set_context("hello");
		expect(get_context::<&str>()).to_be("hello");

		expect(get_context::<i32>()).to_be(7);
	}
}
