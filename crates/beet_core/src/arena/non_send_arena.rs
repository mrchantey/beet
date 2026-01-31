//! Thread-local arena for storing non-`Send` objects.
//!
//! This module provides arena storage for types that cannot be sent across
//! thread boundaries, such as types containing `Rc` or raw pointers.

use std::any::Any;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Entry in the arena that tracks reference count.
struct NonSendArenaEntry {
	object: Box<dyn Any>,
	ref_count: usize,
}

/// A thread-local arena for storing non-`Send` objects.
///
/// Unlike [`Arena`](super::Arena), this arena uses thread-local storage and
/// does not require `Send` bounds on stored types. Objects are only accessible
/// from the thread that created them.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// # use std::rc::Rc;
/// NonSendArena::clear();
///
/// // Rc is not Send, but can be stored here
/// let handle = NonSendArena::insert(Rc::new(42));
/// assert_eq!(*handle.get(), Rc::new(42));
/// drop(handle); // automatically cleaned up
/// ```
pub struct NonSendArena;

impl NonSendArena {
	thread_local! {
		static ARENA: LazyLock<NonSendArenaMap> = LazyLock::new(|| NonSendArenaMap::new());
	}

	/// Provides access to the thread-local arena.
	pub fn with<F, R>(
		&'static self,
		func: impl FnOnce(&LazyLock<NonSendArenaMap>) -> R,
	) -> R {
		Self::ARENA.with(func)
	}

	/// Inserts an object into the arena and returns a reference-counted handle.
	pub fn insert<T: 'static>(object: T) -> NonSendRcArenaHandle<T> {
		Self::ARENA.with(|arena| arena.insert(object))
	}

	/// Returns the number of objects stored in the arena.
	pub fn len() -> usize { Self::ARENA.with(|arena| arena.len()) }

	/// Removes all objects from the arena, invalidating all handles.
	pub fn clear() { Self::ARENA.with(|arena| arena.clear()) }

	/// Returns `true` if the arena contains no objects.
	pub fn is_empty() -> bool { Self::ARENA.with(|arena| arena.is_empty()) }
}

/// The underlying storage for [`NonSendArena`].
///
/// Stores heterogeneous non-`Send` objects with automatic cleanup via
/// reference counting.
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

	/// Stores an object and returns a handle to it.
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

	/// Increments the reference count for a handle.
	fn inc_ref(&self, id: usize) -> bool {
		let mut objects = self.objects.borrow_mut();
		if let Some(entry) = objects.get_mut(&id) {
			entry.ref_count += 1;
			true
		} else {
			false
		}
	}

	/// Decrements the reference count and removes if it reaches zero.
	fn dec_ref(&self, id: usize) {
		let mut objects = self.objects.borrow_mut();
		if let Some(entry) = objects.get_mut(&id) {
			entry.ref_count -= 1;
			if entry.ref_count == 0 {
				objects.remove(&id);
			}
		}
	}

	/// Returns a reference to an object by its handle.
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

	/// Returns a mutable reference to an object by its handle.
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

	/// Manually removes an object from the arena.
	pub fn remove<H: NonSendHandle>(
		&self,
		handle: &H,
	) -> Option<H::ObjectType> {
		let mut objects = self.objects.borrow_mut();

		objects
			.remove(&handle.id())
			.and_then(|entry| entry.object.downcast().ok())
			.map(|boxed| *boxed)
	}

	/// Returns the number of objects stored.
	pub fn len(&self) -> usize { self.objects.borrow().len() }

	/// Returns `true` if the arena is empty.
	pub fn is_empty(&self) -> bool { self.objects.borrow().is_empty() }

	/// Clears all objects from the arena.
	pub fn clear(&self) { self.objects.borrow_mut().clear(); }

	/// Returns the reference count for debugging.
	pub fn ref_count(&self, id: usize) -> Option<usize> {
		self.objects.borrow().get(&id).map(|entry| entry.ref_count)
	}
}

const PANIC_MSG: &str = r#"
Object does not exist in the NonSendArena.
It may have been manually removed by another handle, or created in a different thread.
"#;

/// Trait for handles that provide access to objects in the non-send arena.
pub trait NonSendHandle: Sized {
	/// The type of object this handle refers to.
	type ObjectType: 'static;

	/// Returns the internal ID of this handle.
	fn id(&self) -> usize;

	/// Returns a reference to the arena this handle belongs to.
	fn get_arena(&self) -> &NonSendArenaMap;

	/// Returns a reference to the object in the arena.
	///
	/// # Panics
	///
	/// Panics if the object has been manually removed.
	fn get(&self) -> Ref<'_, Self::ObjectType> {
		self.get_arena().get(self).expect(PANIC_MSG)
	}

	/// Returns a mutable reference to the object in the arena.
	///
	/// # Panics
	///
	/// Panics if the object has been manually removed.
	fn get_mut(&self) -> RefMut<'_, Self::ObjectType> {
		self.get_arena().get_mut(self).expect(PANIC_MSG)
	}

	/// Removes the object from the arena and returns it.
	///
	/// This invalidates all other handles to the same object.
	///
	/// # Panics
	///
	/// Panics if the object has already been manually removed.
	fn remove(self) -> Self::ObjectType {
		self.get_arena().remove(&self).expect(PANIC_MSG)
	}

	/// Returns the current reference count for this handle's object.
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

/// A reference-counted handle to an object in the non-send arena.
///
/// When all clones of this handle are dropped, the object is automatically
/// removed from the arena. Use [`forget`](Self::forget) to convert to a
/// non-reference-counted handle if automatic cleanup is not desired.
pub struct NonSendRcArenaHandle<T: 'static> {
	id: usize,
	arena: *const NonSendArenaMap,
	_phantom: std::marker::PhantomData<T>,
}

impl<T> NonSendRcArenaHandle<T> {
	/// Converts this handle to a non-reference-counted version.
	///
	/// The returned handle will not automatically clean up the object when
	/// dropped. Use this when you need to manually control the object's
	/// lifetime.
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

/// A copyable handle to an object in the non-send arena.
///
/// Unlike [`NonSendRcArenaHandle`], this handle does not automatically clean up
/// the object when dropped. The object must be manually removed or will leak.
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

	#[test]
	fn automatic_cleanup() {
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
	fn multiple_objects_cleanup() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("first".to_string(), 1));
		let handle2 =
			NonSendArena::insert(NonSendCounter::new("second".to_string(), 2));
		let handle3 = NonSendArena::insert("string".to_string());

		NonSendArena::len().xpect_eq(3);

		let handle1_clone = handle1.clone();
		handle1.ref_count().xpect_eq(2);

		drop(handle1);
		NonSendArena::len().xpect_eq(3);

		drop(handle2);
		NonSendArena::len().xpect_eq(2);

		drop(handle3);
		NonSendArena::len().xpect_eq(1);

		drop(handle1_clone);
		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	fn manual_remove_with_clones() {
		NonSendArena::clear();

		let handle1 =
			NonSendArena::insert(NonSendCounter::new("test".to_string(), 42));
		let _handle2 = handle1.clone();

		NonSendArena::len().xpect_eq(1);
		handle1.ref_count().xpect_eq(2);

		let removed = handle1.remove();
		removed.get_name().xpect_eq("test");
		NonSendArena::len().xpect_eq(0);
	}

	#[test]
	fn basic_arena_operations() {
		NonSendArena::clear();

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
	fn forget_functionality() {
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
	fn panic_on_invalid_handle_access() {
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
	fn panic_on_invalid_handle_get_mut() {
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
	fn panic_on_invalid_handle_remove() {
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
