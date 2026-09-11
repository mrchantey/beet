//! Newtype wrapper around an insertion-ordered [`IndexMap`] of [`Value`]s,
//! providing ergonomic key access and a deterministic [`Hash`] implementation.
use crate::prelude::*;
use indexmap::IndexMap;

/// A map of string keys to [`Value`]s.
///
/// Provides ergonomic access with `&str` keys and fallible getters for
/// use in fallible functions via `?`.
///
/// Entries keep their **insertion order**, so a map read from a file iterates
/// as the file did. A scene's `entities` map relies on that: its order is child
/// order (`template_serde`), a fact a hashed map would silently drop. Equality
/// and [`Hash`] ignore order, and the generic serde form still sorts keys, so
/// two maps holding the same entries stay equal however they were built.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", derive(::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Map(pub IndexMap<SmolStr, Value, FixedHasher>);

/// Serializes entries sorted by key, matching the deterministic
/// [`Hash`], [`Ord`] and [`Display`] impls rather than insertion order.
#[cfg(feature = "serde")]
impl ::serde::Serialize for Map {
	fn serialize<S: ::serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		use ::serde::ser::SerializeMap;
		let mut entries: Vec<_> = self.0.iter().collect();
		entries.sort_by_key(|(key, _)| key.as_str());
		let mut map = serializer.serialize_map(Some(entries.len()))?;
		for (key, value) in entries {
			map.serialize_entry(key, value)?;
		}
		map.end()
	}
}

impl core::hash::Hash for Map {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		// sort entries for deterministic hashing regardless of insertion order
		let mut entries: Vec<_> = self.0.iter().collect();
		entries.sort_by_key(|(key, _)| key.as_str());
		for (key, value) in entries {
			key.hash(state);
			value.hash(state);
		}
	}
}

impl Ord for Map {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		let mut self_entries: Vec<_> = self.0.iter().collect();
		self_entries.sort_by_key(|(key, _)| key.as_str());

		let mut other_entries: Vec<_> = other.0.iter().collect();
		other_entries.sort_by_key(|(key, _)| key.as_str());

		self_entries.cmp(&other_entries)
	}
}

impl PartialOrd for Map {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Map {
	/// Gets a value by key.
	///
	/// ## Errors
	/// Returns an error if the key is not found.
	pub fn get(&self, key: &str) -> Result<&Value> {
		self.0
			.get(key)
			.ok_or_else(|| bevyhow!("key {:?} not found in Map", key))
	}

	/// Returns `true` if the map contains the given key.
	pub fn contains(&self, key: &str) -> bool { self.0.contains_key(key) }

	/// Inserts a key-value pair, overwriting any existing value. A new key lands
	/// last, an existing one keeps its position.
	///
	/// Returns the previous value if the key existed.
	pub fn insert(
		&mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Option<Value> {
		self.0.insert(key.into(), value.into())
	}

	/// Removes a key, keeping the order of the remaining entries.
	///
	/// Returns the removed value if the key existed.
	pub fn remove(&mut self, key: &str) -> Option<Value> {
		self.0.shift_remove(key)
	}

	/// Inserts a key-value pair.
	///
	/// ## Errors
	/// Returns an error if the key already exists.
	pub fn try_insert(
		&mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Result<()> {
		let key = key.into();
		if self.0.contains_key(&key) {
			bevybail!("key {:?} already exists in Map", key)
		}
		self.0.insert(key, value.into());
		Ok(())
	}
}

impl core::fmt::Display for Map {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let mut entries: Vec<_> = self.0.iter().collect();
		entries.sort_by_key(|(key, _)| key.as_str());
		write!(
			f,
			"{{{}}}",
			entries
				.iter()
				.map(|(key, val)| format!("{}: {}", key, val))
				.collect::<Vec<_>>()
				.join(", ")
		)
	}
}

impl From<HashMap<SmolStr, Value>> for Map {
	fn from(map: HashMap<SmolStr, Value>) -> Self { map.into_iter().collect() }
}

impl From<Map> for HashMap<SmolStr, Value> {
	fn from(map: Map) -> Self { map.0.into_iter().collect() }
}

impl IntoIterator for Map {
	type Item = (SmolStr, Value);
	type IntoIter = indexmap::map::IntoIter<SmolStr, Value>;
	fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}

impl<'a> IntoIterator for &'a Map {
	type Item = (&'a SmolStr, &'a Value);
	type IntoIter = indexmap::map::Iter<'a, SmolStr, Value>;
	fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

impl<'a> IntoIterator for &'a mut Map {
	type Item = (&'a SmolStr, &'a mut Value);
	type IntoIter = indexmap::map::IterMut<'a, SmolStr, Value>;
	fn into_iter(self) -> Self::IntoIter { self.0.iter_mut() }
}

impl FromIterator<(SmolStr, Value)> for Map {
	fn from_iter<I: IntoIterator<Item = (SmolStr, Value)>>(iter: I) -> Self {
		Self(iter.into_iter().collect())
	}
}
