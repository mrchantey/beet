use bevy::prelude::*;



/// A simple counter to generate unique ids.
///
/// ## Example
///
/// ```
/// # use bevy::prelude::*;
/// # use beet_core::prelude::*;
///
/// fn my_system(mut counter: Local<Counter>) {
/// // incremented each time the system runs
/// let val = counter.next();
/// assert!(val >= 0);
/// }
///
/// ```
///
#[derive(Debug, Default, Clone, Resource)]
pub struct Counter(u64);
impl Counter {
	/// Get the next unique style id.
	pub fn next(&mut self) -> u64 {
		let id = self.0;
		self.0 += 1;
		id
	}
}
