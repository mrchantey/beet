/// Utility functions for working with [`Vec`].
pub struct VecExt<T> {
	phantom: std::marker::PhantomData<T>,
}

impl<T> VecExt<T> {
	/// Returns a mutable reference to a matching element, or inserts a new one.
	///
	/// Searches `vec` for an element matching `predicate`. If found, returns a
	/// mutable reference to it. Otherwise, inserts `default()` and returns a
	/// mutable reference to the newly inserted element.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let mut vec = vec![("a", 1), ("b", 2)];
	///
	/// // Find existing entry
	/// let entry = VecExt::entry_or_insert_with(
	///     &mut vec,
	///     |(k, _)| *k == "a",
	///     || ("a", 0),
	/// );
	/// assert_eq!(entry, &mut ("a", 1));
	///
	/// // Insert new entry
	/// let entry = VecExt::entry_or_insert_with(
	///     &mut vec,
	///     |(k, _)| *k == "c",
	///     || ("c", 3),
	/// );
	/// assert_eq!(entry, &mut ("c", 3));
	/// assert_eq!(vec.len(), 3);
	/// ```
	pub fn entry_or_insert_with(
		vec: &mut Vec<T>,
		predicate: impl Fn(&T) -> bool,
		default: impl FnOnce() -> T,
	) -> &mut T {
		for i in 0..vec.len() {
			if predicate(&vec[i]) {
				return &mut vec[i];
			}
		}
		vec.push(default());
		vec.last_mut().unwrap()
	}
}
