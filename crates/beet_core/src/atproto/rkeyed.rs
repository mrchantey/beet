use crate::prelude::*;

/// A record body paired with the rkey it lives at: the one shape a write
/// takes, since the key is the record's address and never part of its body.
///
/// Reads through to the body, so a pair is used as the record it carries.
///
/// ```
/// # use beet_core::prelude::*;
/// let note = Rkeyed::new(Rkey::parse("first").unwrap(), String::from("hi"));
/// note.rkey().as_str().xpect_eq("first");
/// note.len().xpect_eq(2);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct Rkeyed<T> {
	rkey: Rkey,
	#[deref]
	value: T,
}

impl<T> Rkeyed<T> {
	/// `value` at `rkey`.
	pub fn new(rkey: Rkey, value: T) -> Self { Self { rkey, value } }

	/// The key the record lives at.
	pub fn rkey(&self) -> &Rkey { &self.rkey }

	/// The record body.
	pub fn into_value(self) -> T { self.value }

	/// The key and the body.
	pub fn into_parts(self) -> (Rkey, T) { (self.rkey, self.value) }
}

impl<T> From<(Rkey, T)> for Rkeyed<T> {
	fn from((rkey, value): (Rkey, T)) -> Self { Self::new(rkey, value) }
}
