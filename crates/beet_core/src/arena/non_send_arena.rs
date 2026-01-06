use std::any::Any;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::sync::LazyLock;

/// A thread-local arena for storing non-Send objects
pub struct NonSendArena;
impl NonSendArena {
	thread_local! {
		static ARENA: LazyLock<NonSendArenaMap> = LazyLock::new(|| NonSendArenaMap::new());
	}
	pub fn with<F, R>(
		&'static self,
		func: impl FnOnce(&LazyLock<NonSendArenaMap>) -> R,
	) -> R {
		Self::ARENA.with(func)
	}
	/// Insert the object into the arena and return a handle to it
	pub fn insert<T: 'static>(object: T) -> NonSendRcArenaHandle<T> {
		Self::ARENA.with(|arena| arena.insert(object))
	}
	/// Get the number of objects stored in the arena
	pub fn len() -> usize { Self::ARENA.with(|arena| arena.len()) }
	/// Remove all objects from the arena, invalidating all handles
	pub fn clear() { Self::ARENA.with(|arena| arena.clear()) }
	/// Check if the arena is empty
	pub fn is_empty() -> bool { Self::ARENA.with(|arena| arena.is_empty()) }
}

/// Entry in the arena that tracks reference count
struct NonSendArenaEntry {
	object: Box<dyn Any>,
	ref_count: usize,
}

/// Arena that stores heterogeneous non-Send objects with automatic cleanup
pub struct NonSendArenaMap {
	objects: RefCell<HashMap<usize, NonSendArenaEntry>>,
	next_id: RefCell<usize>,
}

impl NonSendArenaMap {
	fn new() -> Self {
		Self {
			objects: RefCell::new(HashMap::new()),
			next_id: RefCell::new(0),
		}
	}

	/// Store an object and return a handle to it
	pub fn insert<T: 'static>(&self, object: T) -> NonSendRcArenaHandle<T> {
		let mut objects = self.objects.borrow_mut();
		let mut next_id = self.next_id.borrow_mut();

		let id = *next_id;
		*next_id += 1;

		let entry = NonSendArenaEntry {
			object: Box::new(object),
			ref_count: 1,
		};

		objects.insert(id, entry);

		NonSendRcArenaHandle {
			id,
			arena: self as *const NonSendArenaMap,
			_phantom: std::marker::PhantomData,
		}
	}

	/// Increment reference count for a handle
	fn inc_ref(&self, id: usize) -> bool {
		let mut objects = self.objects.borrow_mut();
		if let Some(entry) = objects.get_mut(&id) {
			entry.ref_count += 1;
			true
		} else {
			false
		}
	}

	/// Decrement reference count and remove if it reaches zero
	fn dec_ref(&self, id: usize) {
		let mut objects = self.objects.borrow_mut();
		if let Some(entry) = objects.get_mut(&id) {
			entry.ref_count -= 1;
			if entry.ref_count == 0 {
				objects.remove(&id);
			}
		}
	}

	/// Get a reference to an object by its handle
	pub fn get<H: NonSendHandle>(
		&self,
		handle: &H,
	) -> Option<Ref<'_, H::ObjectType>> {
		let objects = self.objects.borrow();

		// Check if the object exists and can be downcast to the correct type
		if objects.contains_key(&handle.id()) {
			// We need to use Ref::map to maintain the borrow
			Ref::filter_map(objects, |map| {
				map.get(&handle.id())
					.and_then(|entry| entry.object.downcast_ref())
			})
			.ok()
		} else {
			None
		}
	}

	/// Get a mutable reference to an object by its handle
	pub fn get_mut<H: NonSendHandle>(
		&self,
		handle: &H,
	) -> Option<RefMut<'_, H::ObjectType>> {
		let objects = self.objects.borrow_mut();

		RefMut::filter_map(objects, |map| {
			map.get_mut(&handle.id())
				.and_then(|entry| entry.object.downcast_mut::<H::ObjectType>())
		})
		.ok()
	}

	/// Manually remove an object from the arena (consumes all handles)
	pub fn remove<H: NonSendHandle>(
		&self,
		handle: &H,
	) -> Option<H::ObjectType> {
		let mut objects = self.objects.borrow_mut();
		// Don't call drop on the handle since we're consuming it
		// std::mem::forget(handle);

		objects
			.remove(&handle.id())
			.and_then(|entry| entry.object.downcast().ok())
			.map(|boxed| *boxed)
	}

	/// Get the number of objects stored
	pub fn len(&self) -> usize { self.objects.borrow().len() }

	/// Check if the arena is empty
	pub fn is_empty(&self) -> bool { self.objects.borrow().is_empty() }

	/// Clear all objects from the arena
	pub fn clear(&self) { self.objects.borrow_mut().clear(); }

	/// Get reference count for debugging
	pub fn ref_count(&self, id: usize) -> Option<usize> {
		self.objects.borrow().get(&id).map(|entry| entry.ref_count)
	}
}

const PANIC_MSG: &str = r#"
Object does not exist in the NonSendArena.
It may have been manually removed by another handle, or created in a different thread.
"#;


pub trait NonSendHandle: Sized {
	type ObjectType: 'static;
	fn id(&self) -> usize;
	fn get_arena(&self) -> &NonSendArenaMap;

	/// Get a reference to the object in the arena
	/// ## Panics
	/// Panics if the object has been manually removed.
	fn get(&self) -> Ref<'_, Self::ObjectType> {
		self.get_arena().get(self).expect(PANIC_MSG)
	}

	/// Get a mutable reference to the object in the arena
	/// ## Panics
	/// Panics if the object has been manually removed.
	fn get_mut(&self) -> RefMut<'_, Self::ObjectType> {
		self.get_arena().get_mut(self).expect(PANIC_MSG)
	}

	/// Manually remove the object from the arena.
	/// This will invalidate all other handles.
	/// ## Panics
	/// Panics if the object has already been manually removed.
	fn remove(self) -> Self::ObjectType {
		self.get_arena().remove(&self).expect(PANIC_MSG)
	}
	fn ref_count(&self) -> usize {
		self.get_arena().ref_count(self.id()).expect(PANIC_MSG)
	}
}
impl<T: 'static> NonSendHandle for NonSendRcArenaHandle<T> {
	type ObjectType = T;
	fn id(&self) -> usize { self.id }
	fn get_arena(&self) -> &NonSendArenaMap { unsafe { &*self.arena } }
}
impl<T: 'static> NonSendHandle for NonSendArenaHandle<T> {
	type ObjectType = T;
	fn id(&self) -> usize { self.id }
	fn get_arena(&self) -> &NonSendArenaMap { unsafe { &*self.arena } }
}

// Handle that provides type-safe access to objects in the arena
// Automatically manages reference counting
pub struct NonSendRcArenaHandle<T: 'static> {
	id: usize,
	arena: *const NonSendArenaMap,
	_phantom: std::marker::PhantomData<T>,
}

impl<T> NonSendRcArenaHandle<T> {
	pub fn forget(self) -> NonSendArenaHandle<T> {
		let id = self.id;
		let handle = NonSendArenaHandle {
			id,
			arena: self.arena,
			_phantom: std::marker::PhantomData,
		};
		std::mem::forget(self);
		handle
	}
}

impl<T: 'static> Clone for NonSendRcArenaHandle<T> {
	fn clone(&self) -> Self {
		let arena = self.get_arena();
		if arena.inc_ref(self.id) {
			NonSendRcArenaHandle {
				id: self.id,
				arena: self.arena,
				_phantom: std::marker::PhantomData,
			}
		} else {
			// Handle is invalid, but we still need to return something
			// This shouldn't happen in normal usage
			panic!("Attempted to clone invalid handle");
		}
	}
}

impl<T: 'static> Drop for NonSendRcArenaHandle<T> {
	fn drop(&mut self) {
		let arena = self.get_arena();
		arena.dec_ref(self.id);
	}
}

/// A `Copy` version of the `RcArenaHandle` that doesn't automatically clean up.
#[derive(Clone, Copy)]
pub struct NonSendArenaHandle<T> {
	id: usize,
	arena: *const NonSendArenaMap,
	_phantom: std::marker::PhantomData<T>,
}

#[cfg(test)]
mod tests {
	use std::rc::Rc;

	use super::*;
	use crate::prelude::*;
	// Example non-Send type for demonstration
	#[derive(Debug)]
	struct NonSendCounter {
		value: Rc<RefCell<i32>>,
		name: String,
	}

	impl NonSendCounter {
		fn new(name: String, initial_value: i32) -> Self {
			Self {
				value: Rc::new(RefCell::new(initial_value)),
				name,
			}
		}

		fn increment(&self) { *self.value.borrow_mut() += 1; }

		fn get_value(&self) -> i32 { *self.value.borrow() }

		fn get_name(&self) -> &str { &self.name }
	}

	impl Drop for NonSendCounter {
		fn drop(&mut self) {
			// println!("NonSendCounter '{}' is being dropped", self.name);
		}
	}

	#[test]
	fn handle_is_send() {
		// Check if the handle is Send
		// fn assert_send<T: Send>() {}
		// assert_send::<RcArenaHandle<NonSendCounter>>();
		// assert_send::<ArenaHandle<NonSendCounter>>();
	}
	#[test]
	fn test_automatic_cleanup() {
		NonSendArena::clear();

		// Create a handle and clone it
		{
			let handle1 = NonSendArena::insert(NonSendCounter::new(
				"test".to_string(),
				42,
			));

			NonSendArena::len().xpect_eq(1);
			handle1.ref_count().xpect_eq(1);

			// Clone the handle - should increase ref count
			let handle2 = handle1.clone();
			handle1.ref_count().xpect_eq(2);

			// Drop handle1 - ref count should decrease but object should remain
			drop(handle1);
			handle2.ref_count().xpect_eq(1);
			NonSendArena::len().xpect_eq(1);

			// Object should still be accessible through handle2
			{
				let counter = handle2.get();
				counter.get_value().xpect_eq(42);
			}

			// Drop handle2 - should automatically remove object from arena
		}

		// Object should now be gone
		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	fn test_multiple_objects_cleanup() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("first".to_string(), 1));
		let handle2 =
			NonSendArena::insert(NonSendCounter::new("second".to_string(), 2));
		let handle3 = NonSendArena::insert("string".to_string());

		NonSendArena::len().xpect_eq(3);

		// Clone one handle
		let handle1_clone = handle1.clone();
		handle1.ref_count().xpect_eq(2);

		// Drop original handle1
		drop(handle1);
		NonSendArena::len().xpect_eq(3); // Should still be there due to clone

		// Drop handle2 - should remove that object
		drop(handle2);
		NonSendArena::len().xpect_eq(2);

		// Drop handle3 - should remove that object
		drop(handle3);
		NonSendArena::len().xpect_eq(1);

		// Drop the clone - should remove the last object
		drop(handle1_clone);
		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	fn test_manual_remove_with_clones() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("test".to_string(), 42));
		let _handle2 = handle1.clone();

		NonSendArena::len().xpect_eq(1);
		handle1.ref_count().xpect_eq(2);

		// Manual remove should consume all handles and remove the object
		let removed = handle1.remove();
		removed.get_name().xpect_eq("test");
		NonSendArena::len().xpect_eq(0);

		// handle2 should now be invalid and panic when accessed
		// We can't test this directly since it would panic the test
		// but we can verify the object is gone from the arena
		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	fn test_basic_arena_operations() {
		NonSendArena::clear();

		// Store different types
		let counter_handle =
			NonSendArena::insert(NonSendCounter::new("test".to_string(), 42));
		let string_handle = NonSendArena::insert("Hello, World!".to_string());
		let number_handle = NonSendArena::insert(123i32);

		NonSendArena::len().xpect_eq(3);

		// Access stored objects
		{
			let counter = counter_handle.get();
			counter.get_value().xpect_eq(42);
			counter.get_name().xpect_eq("test");
		}

		{
			let string = string_handle.get();
			string.as_str().xpect_eq("Hello, World!");
		}

		{
			let number = number_handle.get();
			(*number).xpect_eq(123);
		}

		// Mutate objects
		{
			let counter = counter_handle.get();
			counter.increment();
			counter.get_value().xpect_eq(43);
		}

		// Objects should be automatically cleaned up when handles are dropped
		drop(counter_handle);
		drop(string_handle);
		drop(number_handle);

		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	fn test_forget_functionality() {
		NonSendArena::clear();

		let forgotten_handle;

		// Create a handle and forget it
		{
			let handle = NonSendArena::insert(NonSendCounter::new(
				"forgotten".to_string(),
				100,
			));

			NonSendArena::len().xpect_eq(1);
			handle.ref_count().xpect_eq(1);

			// Forget the handle - should return a copy version that doesn't auto-cleanup
			forgotten_handle = handle.forget();

			// The object should still be in the arena after forgetting
			NonSendArena::len().xpect_eq(1);
			forgotten_handle.ref_count().xpect_eq(1);
		}

		// After the scope ends, the object should still be there because we forgot the handle
		NonSendArena::len().xpect_eq(1);
		forgotten_handle.ref_count().xpect_eq(1);

		// The forgotten handle should still be able to access the object
		{
			let counter = forgotten_handle.get();
			counter.get_value().xpect_eq(100);
			counter.get_name().xpect_eq("forgotten");
		}

		// Since the handle was forgotten, it won't auto-cleanup when dropped
		drop(forgotten_handle);
		NonSendArena::len().xpect_eq(1);

		// We need to manually clear to clean up the forgotten object
		NonSendArena::clear();
		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	#[should_panic]
	fn test_panic_on_invalid_handle_access() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("test".to_string(), 42));
		let handle2 = handle1.clone();

		// Manual remove should consume all handles and remove the object
		let _removed = handle1.remove();
		NonSendArena::len().xpect_eq(0);

		// handle2 should now panic when accessed
		let _counter = handle2.get();
	}

	#[test]
	#[should_panic]
	fn test_panic_on_invalid_handle_get_mut() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("test".to_string(), 42));
		let handle2 = handle1.clone();

		// Manual remove should consume all handles and remove the object
		let _removed = handle1.remove();
		NonSendArena::len().xpect_eq(0);

		// handle2 should now panic when accessed mutably
		let _counter = handle2.get_mut();
	}

	#[test]
	#[should_panic]
	fn test_panic_on_invalid_handle_remove() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("test".to_string(), 42));
		let handle2 = handle1.clone();

		// Manual remove should consume all handles and remove the object
		let _removed = handle1.remove();
		NonSendArena::len().xpect_eq(0);

		// handle2 should now panic when trying to remove again
		let _removed2 = handle2.remove();
	}
}
