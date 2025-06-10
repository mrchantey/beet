use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;

/// Entry in the arena
struct ArenaEntry {
	object: Box<dyn Any + Send>,
}

/// A global arena for storing Send objects with Copy handles
pub struct Arena {
	objects: Arc<Mutex<HashMap<usize, ArenaEntry>>>,
	next_id: Arc<Mutex<usize>>,
}

impl Arena {
	fn new() -> Self {
		Self {
			objects: Arc::new(Mutex::new(HashMap::new())),
			next_id: Arc::new(Mutex::new(0)),
		}
	}

	fn get_global() -> &'static Arena {
		static ARENA: LazyLock<Arena> = LazyLock::new(|| Arena::new());
		&ARENA
	}

	/// Insert the object into the arena and return a handle to it
	pub fn insert<T: Send + 'static>(object: T) -> ArenaHandle<T> {
		Self::get_global().insert_impl(object)
	}

	/// Store an object and return a handle to it
	fn insert_impl<T: Send + 'static>(&self, object: T) -> ArenaHandle<T> {
		let mut objects = self.objects.lock().unwrap();
		let mut next_id = self.next_id.lock().unwrap();

		let id = *next_id;
		*next_id += 1;

		let entry = ArenaEntry {
			object: Box::new(object),
		};

		objects.insert(id, entry);

		ArenaHandle {
			id,
			_phantom: std::marker::PhantomData,
		}
	}

	/// Manually remove an object from the arena
	fn remove_impl<T: 'static>(&self, handle: &ArenaHandle<T>) -> Option<T> {
		let mut objects = self.objects.lock().unwrap();
		objects
			.remove(&handle.id)
			.and_then(|entry| entry.object.downcast().ok())
			.map(|boxed| *boxed)
	}

	/// Get the number of objects stored in the arena
	pub fn len() -> usize { Self::get_global().objects.lock().unwrap().len() }

	/// Remove all objects from the arena, invalidating all handles
	pub fn clear() { Self::get_global().objects.lock().unwrap().clear(); }

	/// Check if the arena is empty
	pub fn is_empty() -> bool {
		Self::get_global().objects.lock().unwrap().is_empty()
	}
}

const PANIC_MSG: &str = r#"
Object does not exist in the Arena. 
It may have been manually removed by another handle.
"#;

/// A `Copy` handle that provides type-safe access to objects in the arena
#[derive(Clone, Copy)]
pub struct ArenaHandle<T> {
	id: usize,
	_phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + 'static> ArenaHandle<T> {
	/// Get a clone of the object
	/// ## Panics  
	/// Panics if the object has been removed
	pub fn get_cloned(&self) -> T {
		let objects = Arena::get_global().objects.lock().unwrap();
		objects
			.get(&self.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.cloned()
			.expect(PANIC_MSG)
	}
}

impl<T: 'static> ArenaHandle<T> {
	/// Execute a function with a reference to the object
	/// ## Panics
	/// Panics if the object has been removed
	pub fn with<R>(&self, func: impl FnOnce(&T) -> R) -> R {
		let objects = Arena::get_global().objects.lock().unwrap();
		let obj = objects
			.get(&self.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.expect(PANIC_MSG);
		func(obj)
	}

	/// Execute a function with a mutable reference to the object
	/// ## Panics
	/// Panics if the object has been removed
	pub fn with_mut<R>(&self, func: impl FnOnce(&mut T) -> R) -> R {
		let mut objects = Arena::get_global().objects.lock().unwrap();
		let obj = objects
			.get_mut(&self.id)
			.and_then(|entry| entry.object.downcast_mut::<T>())
			.expect(PANIC_MSG);
		func(obj)
	}

	/// Manually remove the object from the arena.
	/// This will invalidate all other handles.
	/// ## Panics
	/// Panics if the object has already been manually removed.
	pub fn remove(self) -> T {
		Arena::get_global().remove_impl(&self).expect(PANIC_MSG)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Example Send type for demonstration
	#[derive(Debug, Clone)]
	struct Counter {
		value: i32,
		name: String,
	}

	impl Counter {
		fn new(name: String, initial_value: i32) -> Self {
			Self {
				value: initial_value,
				name,
			}
		}

		fn increment(&mut self) { self.value += 1; }

		fn get_value(&self) -> i32 { self.value }

		fn get_name(&self) -> &str { &self.name }
	}

	impl Drop for Counter {
		fn drop(&mut self) {
			println!("Counter '{}' is being dropped", self.name);
		}
	}

	#[test]
	#[ignore = "race condition"]
	fn handle_is_copy() {
		// Check if the handle is Copy and Send
		fn assert_copy<T: Copy>() {}
		fn assert_send<T: Send>() {}
		assert_copy::<ArenaHandle<i32>>();
		assert_send::<ArenaHandle<i32>>();
	}

	#[test]
	fn test_basic_arena_operations() {
		Arena::clear();

		// Store different types
		let counter_handle =
			Arena::insert(Counter::new("test".to_string(), 42));
		let string_handle = Arena::insert("Hello, World!".to_string());
		let number_handle = Arena::insert(123i32);

		assert_eq!(Arena::len(), 3);

		// Access stored objects using with()
		counter_handle.with(|counter| {
			assert_eq!(counter.get_value(), 42);
			assert_eq!(counter.get_name(), "test");
		});

		string_handle.with(|string| {
			assert_eq!(string, "Hello, World!");
		});

		number_handle.with(|number| {
			assert_eq!(*number, 123);
		});

		// Mutate objects using with_mut()
		counter_handle.with_mut(|counter| {
			counter.increment();
			assert_eq!(counter.get_value(), 43);
		});

		// Test cloned access for cloneable types
		let cloned_counter = counter_handle.get_cloned();
		assert_eq!(cloned_counter.get_value(), 43);

		// Manual cleanup
		let _removed_counter = counter_handle.remove();
		let _removed_string = string_handle.remove();
		let _removed_number = number_handle.remove();

		assert_eq!(Arena::len(), 0);
	}

	#[test]
	#[ignore = "race condition"]
	fn test_copy_handles() {
		Arena::clear();

		let handle1 = Arena::insert(Counter::new("test".to_string(), 100));

		// Copy the handle explicitly
		let handle2 = handle1.clone();

		// Both handles should work (handle1 is still valid after copy)
		handle1.with(|counter| {
			assert_eq!(counter.get_value(), 100);
		});

		handle2.with(|counter| {
			assert_eq!(counter.get_value(), 100);
		});

		// Modify through one handle
		handle1.with_mut(|counter| {
			counter.increment();
		});

		// See the change through the other handle
		handle2.with(|counter| {
			assert_eq!(counter.get_value(), 101);
		});

		// Remove using one handle (this consumes the handle)
		let removed = handle1.remove();
		assert_eq!(removed.get_value(), 101);
		assert_eq!(Arena::len(), 0);
	}

	#[test]
	#[should_panic]
	#[ignore = "race condition"]
	fn test_panic_on_invalid_handle_access() {
		Arena::clear();

		let handle1 = Arena::insert(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone(); // Copy the handle

		// Manual remove should invalidate all handles
		let _removed = handle2.remove();
		assert_eq!(Arena::len(), 0);

		// handle1 should now panic when accessed
		handle1.with(|_| {});
	}

	#[test]
	#[should_panic]
	#[ignore = "race condition"]
	fn test_panic_on_invalid_handle_with_mut() {
		Arena::clear();

		let handle1 = Arena::insert(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone(); // Copy the handle

		// Manual remove should invalidate all handles
		let _removed = handle2.remove();
		assert_eq!(Arena::len(), 0);

		// handle1 should now panic when accessed mutably
		handle1.with_mut(|_| {});
	}

	#[test]
	#[should_panic]
	#[ignore = "race condition"]
	fn test_panic_on_invalid_handle_remove() {
		Arena::clear();

		let handle1 = Arena::insert(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone(); // Copy the handle

		// Manual remove should invalidate all handles
		let _removed = handle2.remove();
		assert_eq!(Arena::len(), 0);

		// handle1 should now panic when trying to remove again
		let _removed2 = handle1.remove();
	}

	#[test]
	#[ignore = "race condition"]
	fn test_multiple_objects() {
		Arena::clear();

		let handle1 = Arena::insert(Counter::new("first".to_string(), 1));
		let handle2 = Arena::insert(Counter::new("second".to_string(), 2));
		let handle3 = Arena::insert("string".to_string());

		assert_eq!(Arena::len(), 3);

		// All handles should work independently
		handle1.with(|counter| assert_eq!(counter.get_value(), 1));
		handle2.with(|counter| assert_eq!(counter.get_value(), 2));
		handle3.with(|string| assert_eq!(string, "string"));

		// Remove objects individually
		let _removed1 = handle1.remove();
		assert_eq!(Arena::len(), 2);

		let _removed2 = handle2.remove();
		assert_eq!(Arena::len(), 1);

		let _removed3 = handle3.remove();
		assert_eq!(Arena::len(), 0);
	}

	#[test]
	#[ignore = "race condition"]
	fn test_clear_functionality() {
		Arena::clear();

		let _handle1 = Arena::insert(Counter::new("test1".to_string(), 1));
		let _handle2 = Arena::insert(Counter::new("test2".to_string(), 2));
		let _handle3 = Arena::insert("string".to_string());

		assert_eq!(Arena::len(), 3);

		// Clear all objects
		Arena::clear();
		assert_eq!(Arena::len(), 0);
		assert!(Arena::is_empty());
	}
}
