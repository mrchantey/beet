//! [`MultiMap`]: a map holding several values per key, the shape of a query
//! string or an argv (`--tag=a --tag=b`). Parsing one into a reflected type is
//! [`MultiMapReflectExt`](super::MultiMapReflectExt).

use crate::prelude::*;
use core::borrow::Borrow;
use core::hash::BuildHasher;
use core::hash::Hash;

/// A multimap that stores multiple values per key.
///
/// Unlike a standard `HashMap`, this allows multiple values to be associated
/// with the same key. Values are stored in insertion order per key.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
	feature = "serde",
	serde(
		transparent,
		bound(
			serialize = "K: serde::Serialize + Eq + Hash, V: serde::Serialize, S: BuildHasher",
			deserialize = "K: serde::Deserialize<'de> + Eq + Hash, V: serde::Deserialize<'de>, S: BuildHasher + Default"
		)
	)
)]
pub struct MultiMap<K, V, S = FixedHasher> {
	inner: HashMap<K, Vec<V>, S>,
}

impl<K, V, S: Default> Default for MultiMap<K, V, S> {
	fn default() -> Self {
		Self {
			inner: HashMap::default(),
		}
	}
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)>
	for MultiMap<K, V, S>
{
	fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
		let mut map = Self::default();
		for (key, value) in iter {
			map.insert(key, value);
		}
		map
	}
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq
	for MultiMap<K, V, S>
{
	fn eq(&self, other: &Self) -> bool { self.inner == other.inner }
}

impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for MultiMap<K, V, S> {}

impl<K, V, S> core::hash::Hash for MultiMap<K, V, S>
where
	K: Eq + Ord + Hash,
	V: Eq + Hash,
	S: BuildHasher,
{
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		// order of keys doesn't matter, but order of values per key does
		let mut entries: Vec<_> = self.inner.iter().collect();
		entries.sort_by(|a, b| a.0.cmp(b.0)); // sort by key for consistent hashing
		for (key, values) in entries {
			key.hash(state);
			values.hash(state);
		}
	}
}

impl<K, V, S> Ord for MultiMap<K, V, S>
where
	K: Hash + Ord,
	V: Ord,
	S: BuildHasher,
{
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		let mut a: Vec<_> = self.inner.iter().collect();
		let mut b: Vec<_> = other.inner.iter().collect();

		a.sort_by(|x, y| x.0.cmp(y.0));
		b.sort_by(|x, y| x.0.cmp(y.0));

		a.cmp(&b)
	}
}
impl<K, V, S> PartialOrd for MultiMap<K, V, S>
where
	K: Hash + Ord,
	V: Ord,
	S: BuildHasher,
{
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<K, V> MultiMap<K, V>
where
	K: Eq + Hash,
{
	/// Create a new empty multimap.
	pub const fn new() -> Self {
		Self {
			inner: HashMap::new(),
		}
	}
}

impl<K, V, S> MultiMap<K, V, S>
where
	K: Eq + Hash,
	S: BuildHasher + Default,
{
	/// Insert a key with no values.
	/// If the key already exists, this is a no-op.
	pub fn insert_key(&mut self, key: K) { self.inner.entry(key).or_default(); }

	/// Insert a value for a key.
	/// If the key already exists, the value is appended to the existing values.
	pub fn insert(&mut self, key: K, value: V) {
		self.inner.entry(key).or_default().push(value);
	}

	/// Inserts multiple values for a key, appending to any existing values.
	pub fn insert_vec(&mut self, key: K, values: Vec<V>) {
		self.inner.entry(key).or_default().extend(values);
	}

	/// Get the first value for a key.
	pub fn get<Q>(&self, key: &Q) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.get(key).and_then(|values| values.first())
	}

	/// Get the first value matching any of the provided keys.
	pub fn get_multikey<'a, Q>(
		&self,
		keys: impl IntoIterator<Item = &'a Q>,
	) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized + 'a,
	{
		for key in keys.into_iter() {
			if let Some(value) = self.get(key) {
				return Some(value);
			}
		}
		None
	}

	/// Get all values for a key.
	pub fn get_vec<Q>(&self, key: &Q) -> Option<&Vec<V>>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.get(key)
	}

	/// Get a mutable reference to all values for a key.
	pub fn get_vec_mut<Q>(&mut self, key: &Q) -> Option<&mut Vec<V>>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.get_mut(key)
	}

	/// Check if key exists.
	pub fn contains_key<Q>(&self, key: &Q) -> bool
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.contains_key(key)
	}

	/// Returns true if the multimap contains no keys.
	pub fn is_empty(&self) -> bool { self.inner.is_empty() }

	/// Returns the number of keys in the multimap.
	pub fn len(&self) -> usize { self.inner.len() }

	/// Iterate over all key-values pairs.
	pub fn iter_all(&self) -> impl Iterator<Item = (&K, &Vec<V>)> {
		self.inner.iter()
	}

	/// Consume the multimap and iterate over all key-values pairs.
	pub fn into_iter_all(self) -> impl Iterator<Item = (K, Vec<V>)> {
		self.inner.into_iter()
	}

	/// Iterate over all keys.
	pub fn keys(&self) -> impl Iterator<Item = &K> { self.inner.keys() }

	/// Remove a key and all its values.
	pub fn remove<Q>(&mut self, key: &Q) -> Option<Vec<V>>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.remove(key)
	}

	/// Clear all entries.
	pub fn clear(&mut self) { self.inner.clear(); }
}
