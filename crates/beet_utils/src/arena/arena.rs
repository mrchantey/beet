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

	/// Execute a function with a reference to the object in this arena instance.
	/// ## Panics
	/// Panics if the object has been removed or the type doesn't match.
	pub fn with_ref<T: 'static, R>(
		&self,
		handle: &ArenaHandle<T>,
		f: impl FnOnce(&T) -> R,
	) -> R {
		let objects = self.objects.lock().unwrap();
		let obj = objects
			.get(&handle.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.expect(PANIC_MSG);
		f(obj)
	}

	/// Execute a function with a mutable reference to the object in this arena instance.
	/// ## Panics
	/// Panics if the object has been removed or the type doesn't match.
	pub fn with_mut<T: 'static, R>(
		&self,
		handle: &ArenaHandle<T>,
		f: impl FnOnce(&mut T) -> R,
	) -> R {
		let mut objects = self.objects.lock().unwrap();
		let obj = objects
			.get_mut(&handle.id)
			.and_then(|entry| entry.object.downcast_mut::<T>())
			.expect(PANIC_MSG);
		f(obj)
	}

	/// Get a cloned value of a cloneable object stored in this arena instance.
	/// ## Panics
	/// Panics if the object has been removed or the type doesn't match.
	pub fn get_cloned<T: Clone + 'static>(&self, handle: &ArenaHandle<T>) -> T {
		let objects = self.objects.lock().unwrap();
		objects
			.get(&handle.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.cloned()
			.expect(PANIC_MSG)
	}
}

const PANIC_MSG: &str = r#"
Object does not exist in the Arena.
It may have been manually removed by another handle.
"#;

/// A `Copy` handle that provides type-safe access to objects in the arena
pub struct ArenaHandle<T> {
	id: usize,
	_phantom: std::marker::PhantomData<T>,
}

impl<T> Copy for ArenaHandle<T> {}

impl<T> Clone for ArenaHandle<T> {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			_phantom: std::marker::PhantomData,
		}
	}
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
	fn handle_is_copy() {
		// Check if the handle is Copy and Send
		fn assert_copy<T: Copy>() {}
		fn assert_send<T: Send>() {}
		assert_copy::<ArenaHandle<i32>>();
		assert_send::<ArenaHandle<i32>>();
	}

	#[test]
	fn test_basic_arena_operations() {
		let arena = Arena::new();

		// Store different types using the local arena
		let counter_handle =
			arena.insert_impl(Counter::new("test".to_string(), 42));
		let string_handle = arena.insert_impl("Hello, World!".to_string());
		let number_handle = arena.insert_impl(123i32);

		assert_eq!(arena.objects.lock().unwrap().len(), 3);

		// Access stored objects using arena.with_ref()
		arena.with_ref(&counter_handle, |counter| {
			assert_eq!(counter.get_value(), 42);
			assert_eq!(counter.get_name(), "test");
		});

		arena.with_ref(&string_handle, |string| {
			assert_eq!(string, "Hello, World!");
		});

		arena.with_ref(&number_handle, |number| {
			assert_eq!(*number, 123);
		});

		// Mutate objects using arena.with_mut()
		arena.with_mut(&counter_handle, |counter| {
			counter.increment();
			assert_eq!(counter.get_value(), 43);
		});

		// Test cloned access for cloneable types
		let cloned_counter = arena.get_cloned(&counter_handle);
		assert_eq!(cloned_counter.get_value(), 43);

		// Manual cleanup using instance remove_impl
		let _removed_counter =
			arena.remove_impl(&counter_handle).expect(PANIC_MSG);
		let _removed_string =
			arena.remove_impl(&string_handle).expect(PANIC_MSG);
		let _removed_number =
			arena.remove_impl(&number_handle).expect(PANIC_MSG);

		assert_eq!(arena.objects.lock().unwrap().len(), 0);
	}

	#[test]
	fn test_copy_handles() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 100));

		// Copy the handle explicitly
		let handle2 = handle1.clone();

		// Both handles should work (handle1 is still valid after copy)
		arena.with_ref(&handle1, |counter| {
			assert_eq!(counter.get_value(), 100);
		});

		arena.with_ref(&handle2, |counter| {
			assert_eq!(counter.get_value(), 100);
		});

		// Modify through one handle
		arena.with_mut(&handle1, |counter| {
			counter.increment();
		});

		// See the change through the other handle
		arena.with_ref(&handle2, |counter| {
			assert_eq!(counter.get_value(), 101);
		});

		// Remove using one handle (this consumes the stored object)
		let removed = arena.remove_impl(&handle1).expect(PANIC_MSG);
		assert_eq!(removed.get_value(), 101);
		assert_eq!(arena.objects.lock().unwrap().len(), 0);
	}

	#[test]
	#[should_panic]
	fn test_panic_on_invalid_handle_access() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone(); // Copy the handle

		// Manual remove should invalidate the entry in this local arena
		let _removed = arena.remove_impl(&handle2).expect(PANIC_MSG);
		assert_eq!(arena.objects.lock().unwrap().len(), 0);

		// handle1 should now panic when accessed
		arena.with_ref(&handle1, |_| {});
	}

	#[test]
	#[should_panic]
	fn test_panic_on_invalid_handle_with_mut() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone(); // Copy the handle

		// Manual remove should invalidate the entry in this local arena
		let _removed = arena.remove_impl(&handle2).expect(PANIC_MSG);
		assert_eq!(arena.objects.lock().unwrap().len(), 0);

		// handle1 should now panic when accessed mutably
		arena.with_mut(&handle1, |_| {});
	}

	#[test]
	#[should_panic]
	fn test_panic_on_invalid_handle_remove() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone(); // Copy the handle

		// Manual remove should invalidate the entry in this local arena
		let _removed = arena.remove_impl(&handle2).expect(PANIC_MSG);
		assert_eq!(arena.objects.lock().unwrap().len(), 0);

		// handle1 should now panic when trying to remove again
		let _removed2 = arena.remove_impl(&handle1).expect(PANIC_MSG);
	}

	#[test]
	fn test_multiple_objects() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("first".to_string(), 1));
		let handle2 = arena.insert_impl(Counter::new("second".to_string(), 2));
		let handle3 = arena.insert_impl("string".to_string());

		assert_eq!(arena.objects.lock().unwrap().len(), 3);

		// All handles should work independently
		arena.with_ref(&handle1, |counter| assert_eq!(counter.get_value(), 1));
		arena.with_ref(&handle2, |counter| assert_eq!(counter.get_value(), 2));
		arena.with_ref(&handle3, |string| assert_eq!(string, "string"));

		// Remove objects individually
		let _removed1 = arena.remove_impl(&handle1).expect(PANIC_MSG);
		assert_eq!(arena.objects.lock().unwrap().len(), 2);

		let _removed2 = arena.remove_impl(&handle2).expect(PANIC_MSG);
		assert_eq!(arena.objects.lock().unwrap().len(), 1);

		let _removed3 = arena.remove_impl(&handle3).expect(PANIC_MSG);
		assert_eq!(arena.objects.lock().unwrap().len(), 0);
	}

	#[test]
	fn test_clear_functionality() {
		let arena = Arena::new();

		let _handle1 = arena.insert_impl(Counter::new("test1".to_string(), 1));
		let _handle2 = arena.insert_impl(Counter::new("test2".to_string(), 2));
		let _handle3 = arena.insert_impl("string".to_string());

		assert_eq!(arena.objects.lock().unwrap().len(), 3);

		// Clear all objects in the local arena
		arena.objects.lock().unwrap().clear();
		assert_eq!(arena.objects.lock().unwrap().len(), 0);
		assert!(arena.objects.lock().unwrap().is_empty());
	}
}
