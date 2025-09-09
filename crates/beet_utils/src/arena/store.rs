use crate::prelude::*;
use std::fmt::Debug;

/// A small, copyable handle to a value stored in a global Arena.
/// If you need reactivity see [`Signal`]
///
/// The handle itself is `Copy` and `Clone`, so you can pass it around cheaply. All
/// handles point to the same underlying value; mutating through one handle is visible
/// to all others.
///
/// ## Warning
/// This handle must be manually removed from the Arena by calling [`Store::remove`].
/// In short-lived contexts (e.g., tests) leaking is usually harmless, but in
/// long-running applications you should remove it when the value is no longer needed.
///
/// ## Examples
/// Basic usage:
/// ```rust
/// # use beet_utils::prelude::*;
/// let number_store = Store::new(10u32);
/// assert_eq!(number_store.get(), 10);
/// number_store.set(11);
/// assert_eq!(number_store.get(), 11);
/// // Clean up in long-running apps
/// number_store.remove();
/// ```
///
/// Borrow without cloning:
/// ```rust
/// # use beet_utils::prelude::*;
/// let text_store = Store::new(String::from("hello"));
/// let length = text_store.with(|s| s.len());
/// assert_eq!(length, 5);
/// ```
pub struct Store<T>(ArenaHandle<T>);

impl<T> Copy for Store<T> {}

impl<T> Clone for Store<T> {
	fn clone(&self) -> Self { *self }
}

impl<T: 'static + Send + Default> Default for Store<T> {
	/// Creates a `Store<T>` using `T::default()`.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let vec_store = Store::<Vec<u32>>::default();
	/// assert!(vec_store.is_empty());
	/// vec_store.remove();
	/// ```
	fn default() -> Self { Self::new(T::default()) }
}

impl<T: 'static + Send> Store<T> {
	/// Inserts `value` into the Arena and returns a `Store` handle to it.
	///
	/// The underlying allocation is shared by all copies of this handle.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let config_store = Store::new(42u64);
	/// assert_eq!(config_store.get(), 42);
	/// ```
	pub fn new(val: T) -> Self { Self(Arena::insert(val)) }

	/// Returns a clone of the inner value.
	///
	/// If cloning is expensive, prefer [`with`](Self::with) to borrow the value without cloning.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let name_store = Store::new(String::from("beet"));
	/// let cloned = name_store.get();
	/// assert_eq!(cloned, "beet");
	/// ```
	pub fn get(&self) -> T
	where
		T: Clone,
	{
		self.0.with(|val| val.clone())
	}

	/// Replaces the inner value with `value`.
	///
	/// All existing handles will observe the new value.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let flag_store = Store::new(false);
	/// flag_store.set(true);
	/// assert!(flag_store.get());
	/// ```
	pub fn set(&self, value: T) { self.0.with_mut(|val| *val = value); }

	/// Borrows the inner value immutably and applies `f`, returning its result.
	///
	/// This avoids cloning the value.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let numbers = Store::new(vec![1, 2, 3, 4]);
	/// let sum = numbers.with(|v| v.iter().sum::<i32>());
	/// assert_eq!(sum, 10);
	/// ```
	pub fn with<F, R>(&self, f: F) -> R
	where
		F: FnOnce(&T) -> R,
	{
		self.0.with(f)
	}

	/// Manually remove the object from the arena.
	///
	/// After removal, all other `Store<T>` handles to the same object become invalid,
	/// and using them will panic on the next access.
	///
	/// ## Warning
	/// This is the only way to release the allocation backing this store. Consider
	/// calling it when the value is no longer needed in long-running applications.
	///
	/// ## Panics
	/// Panics if the object has already been manually removed.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let count = Store::new(1u32);
	/// count.remove();
	/// // Do not use `count` after this point.
	/// ```
	pub fn remove(&self) { self.0.remove(); }
}


impl<T: 'static + Debug> Debug for Store<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.with(|val| write!(f, "Store({:?})", val))
	}
}


impl<T: 'static> Store<Vec<T>> {
	/// Appends `value` to the end of the inner `Vec<T>`.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let list = Store::<Vec<i32>>::default();
	/// list.push(5);
	/// assert_eq!(list.len(), 1);
	/// ```
	pub fn push(&self, value: T) { self.0.with_mut(|vec| vec.push(value)); }

	/// Removes the last element from the inner `Vec<T>` and returns it, or `None` if empty.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let values = Store::<Vec<i32>>::default();
	/// assert_eq!(values.pop(), None);
	/// values.push(7);
	/// assert_eq!(values.pop(), Some(7));
	/// ```
	pub fn pop(&self) -> Option<T> { self.0.with_mut(|vec| vec.pop()) }

	/// Clears the inner `Vec<T>`, removing all elements.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let messages = Store::<Vec<&'static str>>::default();
	/// messages.push("hi");
	/// messages.clear();
	/// assert!(messages.is_empty());
	/// ```
	pub fn clear(&self) { self.0.with_mut(|vec| vec.clear()); }

	/// Returns the number of elements in the inner `Vec<T>`.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let items = Store::<Vec<i32>>::default();
	/// assert_eq!(items.len(), 0);
	/// items.push(1);
	/// items.push(2);
	/// assert_eq!(items.len(), 2);
	/// ```
	pub fn len(&self) -> usize { self.0.with(|vec| vec.len()) }

	/// Returns `true` if the inner `Vec<T>` contains no elements.
	///
	/// ## Examples
	/// ```rust
	/// # use beet_utils::prelude::*;
	/// let rows = Store::<Vec<u8>>::default();
	/// assert!(rows.is_empty());
	/// rows.push(1);
	/// assert!(!rows.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool { self.0.with(|vec| vec.is_empty()) }
}




#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn works() {
		let store = Store::new(0);

		assert_eq!(store.get(), 0);
		store.set(1);
		assert_eq!(store.get(), 1);
	}

	#[test]
	fn vec() {
		let store = Store::<Vec<u32>>::default();

		assert_eq!(store.len(), 0);
		store.push(1);
		assert_eq!(store.len(), 1);
		store.push(2);
		assert_eq!(store.len(), 2);
		store.pop();
		assert_eq!(store.len(), 1);
		store.clear();
		assert_eq!(store.len(), 0);
	}
}
