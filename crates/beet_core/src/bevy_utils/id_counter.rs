use crate::prelude::*;



/// A simple counter to generate unique ids.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
///
/// fn my_system(mut counter: Local<IdCounter>) {
/// // incremented each time the system runs
/// let val = counter.next();
/// assert!(val >= 0);
/// }
///
/// ```
///
#[derive(Debug, Default, Clone, Resource)]
pub struct IdCounter(u64);
impl IdCounter {
	/// Get the next unique style id.
	pub fn next(&mut self) -> u64 {
		let id = self.0;
		self.0 += 1;
		id
	}
}
