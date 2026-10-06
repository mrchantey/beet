use crate::prelude::*;

/// A record body paired with the rkey it lives at: the one shape a write
/// takes, since the key is the record's address and never part of its body.
///
/// Reads through to the body, so a pair is used as the record it carries, and
/// transforms with [`map`](Self::map) and [`try_map`](Self::try_map) so the
/// key is never separated from what it addresses.
///
/// ```
/// # use beet_core::prelude::*;
/// let note = Rkeyed::new(Rkey::parse("first").unwrap(), String::from("hi"));
/// note.rkey().as_str().xpect_eq("first");
/// note.len().xpect_eq(2);
/// note.map(|text| text.len()).xmap(|len| *len).xpect_eq(2);
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

	/// The same key over a borrow of the body.
	pub fn as_ref(&self) -> Rkeyed<&T> {
		Rkeyed::new(self.rkey.clone(), &self.value)
	}

	/// The same key over `func` of the body.
	pub fn map<U>(self, func: impl FnOnce(T) -> U) -> Rkeyed<U> {
		Rkeyed::new(self.rkey, func(self.value))
	}

	/// The same key over `func` of the body, or its error.
	pub fn try_map<U>(
		self,
		func: impl FnOnce(T) -> Result<U>,
	) -> Result<Rkeyed<U>> {
		Rkeyed::new(self.rkey, func(self.value)?).xok()
	}
}
