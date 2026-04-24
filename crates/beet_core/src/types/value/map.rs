//! Newtype wrapper around [`HashMap<SmolStr, Value>`] providing ergonomic
//! key access and a deterministic [`Hash`] implementation.
use crate::prelude::*;

/// A map of string keys to [`Value`]s.
///
/// Provides ergonomic access with `&str` keys and fallible getters for
/// use in fallible functions via `?`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Map(pub HashMap<SmolStr, Value>);

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

	/// Inserts a key-value pair, overwriting any existing value.
	///
	/// Returns the previous value if the key existed.
	pub fn insert(
		&mut self,
		key: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Option<Value> {
		self.0.insert(key.into(), value.into())
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


impl std::fmt::Display for Map {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
	fn from(map: HashMap<SmolStr, Value>) -> Self { Self(map) }
}

impl From<Map> for HashMap<SmolStr, Value> {
	fn from(map: Map) -> Self { map.0 }
}

impl IntoIterator for Map {
	type Item = (SmolStr, Value);
	type IntoIter = <HashMap<SmolStr, Value> as IntoIterator>::IntoIter;
	fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}

impl<'a> IntoIterator for &'a Map {
	type Item = (&'a SmolStr, &'a Value);
	type IntoIter = <&'a HashMap<SmolStr, Value> as IntoIterator>::IntoIter;
	fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

impl<'a> IntoIterator for &'a mut Map {
	type Item = (&'a SmolStr, &'a mut Value);
	type IntoIter = <&'a mut HashMap<SmolStr, Value> as IntoIterator>::IntoIter;
	fn into_iter(self) -> Self::IntoIter { self.0.iter_mut() }
}

impl FromIterator<(SmolStr, Value)> for Map {
	fn from_iter<I: IntoIterator<Item = (SmolStr, Value)>>(iter: I) -> Self {
		Self(iter.into_iter().collect())
	}
}
