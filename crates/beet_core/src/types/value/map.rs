//! Newtype wrapper around an insertion-ordered [`IndexMap`] of [`Value`]s,
//! providing ergonomic key access and a deterministic [`Hash`] implementation.
use crate::prelude::*;
use core::hash::BuildHasher;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use foldhash::SharedSeed;
use foldhash::fast::FoldHasher;
use foldhash::fast::SeedableRandomState;
use indexmap::IndexMap;

/// A map of string keys to [`Value`]s.
///
/// Provides ergonomic access with `&str` keys and fallible getters for
/// use in fallible functions via `?`.
///
/// Entries keep their **insertion order**, so a map read from a file iterates
/// as the file did. A scene's `entities` map relies on that: its order is child
/// order ([`SceneBody`]), a fact a hashed map would silently drop. Equality
/// and [`Hash`] ignore order, and the generic serde form still sorts keys, so
/// two maps holding the same entries stay equal however they were built.
///
/// Its table is **seeded at random** ([`MapState`]) rather than with the
/// fixed seed of beet's `HashMap`: a map's keys often come from untrusted
/// text, and keys worked out against a known seed would all collide, making
/// one json object quadratic to read. Since iteration follows insertion, the
/// seed changes nothing a caller can observe.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", derive(::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Map(pub IndexMap<SmolStr, Value, MapState>);

/// The hasher of every [`Map`]'s table: foldhash, its secret seed drawn once
/// per process from [`SecureRandom`] (the operating system, or a browser's
/// `crypto.getRandomValues`), so no key can be chosen ahead to collide.
///
/// Without the `rand` feature, or where the platform offers no entropy, the
/// seed is foldhash's own, mixed from addresses and the clock, which on wasm
/// has no clock to mix.
#[derive(Debug, Clone)]
pub struct MapState(SeedableRandomState);

impl MapState {
	/// The secret every table shares, drawn on first use.
	fn shared_seed() -> &'static SharedSeed {
		#[cfg(feature = "rand")]
		{
			use bevy::platform::sync::LazyLock;
			static SECURE: LazyLock<Option<SharedSeed>> = LazyLock::new(|| {
				SecureRandom::new()
					.ok()
					.map(|mut random| u64::from_le_bytes(random.bytes()))
					.map(SharedSeed::from_u64)
			});
			if let Some(seed) = SECURE.as_ref() {
				return seed;
			}
		}
		SharedSeed::global_random()
	}
}

impl Default for MapState {
	fn default() -> Self {
		// a seed per table as well, so no two tables probe alike
		static TABLES: AtomicUsize = AtomicUsize::new(0);
		Self(SeedableRandomState::with_seed(
			TABLES.fetch_add(1, Ordering::Relaxed) as u64,
			Self::shared_seed(),
		))
	}
}

impl BuildHasher for MapState {
	type Hasher = FoldHasher<'static>;
	fn build_hasher(&self) -> FoldHasher<'static> { self.0.build_hasher() }
}

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
	/// A map of `entries`, in order: `Map::new([("name", "ada")])`; values of
	/// mixed types go in as [`Value`].
	pub fn new<K: Into<SmolStr>, V: Into<Value>>(
		entries: impl IntoIterator<Item = (K, V)>,
	) -> Self {
		entries
			.into_iter()
			.map(|(key, value)| (key.into(), value.into()))
			.collect()
	}

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

	/// The number of entries.
	pub fn len(&self) -> usize { self.0.len() }

	/// Whether the map has no entries.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }

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
