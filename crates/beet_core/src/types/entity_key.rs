//! [`EntityKey`]: the key an entity is written under in a scene body.
use crate::prelude::*;

/// The key an entity is written under in a scene body
/// (`template_serde`'s `{ resources, entities }`), and the form every
/// reference to it takes: a component naming another entity holds that
/// entity's key.
///
/// Stable across saves: a document keeps the keys it was read with, and a new
/// entity [mints](Self::minted) the next unused number as its decimal, so a
/// first save reads `"0"`, `"1"`, `"2"` in file order. Any non-empty string is
/// a key, which leaves a text format free to mint its own from structure.
///
/// ```
/// # use beet_core::prelude::*;
/// EntityKey::minted(3).as_str().xpect_eq("3");
/// EntityKey::parse("main/p.3").unwrap().index().xpect_none();
/// EntityKey::parse("").unwrap_err();
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct EntityKey(SmolStr);

impl EntityKey {
	/// Parse a key, refusing only the empty string.
	pub fn parse(text: &str) -> Result<Self> {
		match text.is_empty() {
			true => bevybail!("an entity key is never empty"),
			false => Self(SmolStr::new(text)).xok(),
		}
	}

	/// The key a document mints for the `index`th entity it numbers, its
	/// decimal.
	pub fn minted(index: u32) -> Self { Self(SmolStr::new(index.to_string())) }

	/// The number this key spells, `None` for a key that was not minted.
	pub fn index(&self) -> Option<u32> {
		self.0
			.parse::<u32>()
			.ok()
			// `007` spells no minted key, so it reserves no number
			.filter(|index| *self == Self::minted(*index))
	}

	/// This key as a document writes a reference to its entity.
	pub fn reference(&self) -> Value { Value::Str(self.0.clone()) }

	/// The key `value` references, `None` when it is not a reference.
	pub fn of_reference(value: &Value) -> Option<Self> {
		value.as_str().ok().and_then(|key| Self::parse(key).ok())
	}
}

/// A minted key, `EntityKey::from(3)`.
impl From<u32> for EntityKey {
	fn from(index: u32) -> Self { Self::minted(index) }
}

impl From<&EntityKey> for EntityKey {
	fn from(key: &EntityKey) -> Self { key.clone() }
}

/// A key equals the number it was minted from.
impl PartialEq<u32> for EntityKey {
	fn eq(&self, index: &u32) -> bool { self.index() == Some(*index) }
}

impl PartialEq<str> for EntityKey {
	fn eq(&self, text: &str) -> bool { self.as_str() == text }
}

impl PartialEq<&str> for EntityKey {
	fn eq(&self, text: &&str) -> bool { self.as_str() == *text }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn only_a_minted_key_has_an_index() {
		EntityKey::from(7).index().unwrap().xpect_eq(7);
		EntityKey::parse("07").unwrap().index().xpect_none();
		EntityKey::parse("-1").unwrap().index().xpect_none();
		EntityKey::parse("0/main.0").unwrap().index().xpect_none();
	}

	#[crate::test]
	fn a_reference_is_the_key() {
		let key = EntityKey::from(2);
		key.reference().xpect_eq(Value::str("2"));
		EntityKey::of_reference(&key.reference())
			.unwrap()
			.xpect_eq(key);
		EntityKey::of_reference(&Value::Uint(2)).xpect_none();
	}
}
