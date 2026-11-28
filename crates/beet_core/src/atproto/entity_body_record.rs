//! [`EntityBodyRecord`]: an entity body in the protocol's data model, and
//! [`EntityCollection`], the beet records shaped as one.
use crate::prelude::*;
use alloc::collections::BTreeMap;

/// An entity body in the protocol's data model, which is what a beet record
/// is: components keyed by NSID rather than type path, written flat with the
/// collection as `$type`, every value an [`AtprotoValue`] so floats and bytes
/// take the data model's forms.
///
/// The key is the sole source of truth: a `$type` inside a value is written
/// for a stranger's benefit and never read. Only a [`LexiconDef`] type is read or
/// written by type; any other key is stored untouched, a plugin's component
/// in a binary without the plugin included.
///
/// ```
/// # use beet_core::prelude::*;
/// let mut body = EntityBodyRecord::new(nsid!("org.beet.entity"));
/// body.insert(&Provenance::default()).unwrap();
/// body.get::<Provenance>().unwrap().xpect_eq(Provenance::default());
/// let value = AtprotoValue::from(body.clone());
/// EntityBodyRecord::try_from(value).unwrap().xpect_eq(body);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "Value", into = "AtprotoValue"))]
pub struct EntityBodyRecord {
	collection: Nsid,
	/// Sorted, so a body writes the same bytes whatever order it was built in.
	components: BTreeMap<Nsid, AtprotoValue>,
}

impl EntityBodyRecord {
	/// The key every record writes its collection under.
	pub const TYPE_KEY: &'static str = "$type";

	/// An empty body of `collection`.
	pub fn new(collection: Nsid) -> Self {
		Self {
			collection,
			components: default(),
		}
	}

	/// The collection this body is a record of.
	pub fn collection(&self) -> &Nsid { &self.collection }

	/// This body under another collection, every component untouched: how a
	/// release copies a package record.
	pub fn with_collection(mut self, collection: Nsid) -> Self {
		self.collection = collection;
		self
	}

	/// Every component, by NSID in sorted order.
	pub fn components(&self) -> impl Iterator<Item = (&Nsid, &AtprotoValue)> {
		self.components.iter()
	}

	/// Whether a component is keyed `nsid`.
	pub fn contains(&self, nsid: &Nsid) -> bool {
		self.components.contains_key(nsid)
	}

	/// The component keyed `nsid` as stored.
	pub fn get_value(&self, nsid: &Nsid) -> Option<&AtprotoValue> {
		self.components.get(nsid)
	}

	/// Store `value` under `nsid`, replacing any there.
	pub fn insert_value(&mut self, nsid: Nsid, value: AtprotoValue) {
		self.components.insert(nsid, value);
	}

	/// Remove the component keyed `nsid`, answering it.
	pub fn remove(&mut self, nsid: &Nsid) -> Option<AtprotoValue> {
		self.components.remove(nsid)
	}

	/// The `T` this body holds, an error naming it when absent, so a required
	/// component reads with `?`.
	#[cfg(feature = "serde")]
	pub fn get<T: LexiconDef + DeserializeOwned>(&self) -> Result<T> {
		self.try_get::<T>()?.ok_or_else(|| {
			bevyhow!("`{}` holds no `{}`", self.collection, T::NSID)
		})
	}

	/// The `T` this body holds, `None` when absent.
	#[cfg(feature = "serde")]
	pub fn try_get<T: LexiconDef + DeserializeOwned>(
		&self,
	) -> Result<Option<T>> {
		self.components
			.get(&T::NSID)
			.map(|value| {
				value.clone().into_serde::<T>().map_err(|err| {
					bevyhow!(
						"`{}` in `{}` does not read: {err}",
						T::NSID,
						self.collection
					)
				})
			})
			.transpose()
	}

	/// Store `value` under its NSID, replacing any there.
	#[cfg(feature = "serde")]
	pub fn insert<T: LexiconDef + Serialize>(&mut self, value: &T) -> Result {
		self.components
			.insert(T::NSID, AtprotoValue::from_serde(value)?);
		Ok(())
	}

	/// This body with `value` stored under its NSID.
	#[cfg(feature = "serde")]
	pub fn with<T: LexiconDef + Serialize>(
		mut self,
		value: &T,
	) -> Result<Self> {
		self.insert(value)?;
		self.xok()
	}
}

/// The body written flat: `$type` then every component, in sorted order.
impl From<EntityBodyRecord> for AtprotoValue {
	fn from(body: EntityBodyRecord) -> Self {
		let mut map = Map::default();
		map.insert(
			EntityBodyRecord::TYPE_KEY,
			Value::Str(body.collection.as_str().into()),
		);
		for (nsid, value) in body.components {
			map.insert(nsid.as_str(), value.into_wire());
		}
		// every member was already in the data model
		AtprotoValue::from_wire(Value::Map(map))
			.expect("an entity body record is in the data model")
	}
}

/// A record read back as a beet value, its floats encoded first, so a body
/// decoded on the way to a typed read ([`AtprotoValue::into_serde`]) reads.
impl TryFrom<Value> for EntityBodyRecord {
	type Error = BevyError;
	fn try_from(value: Value) -> Result<Self> {
		AtprotoValue::try_from(value)?.try_into()
	}
}

/// A record read back: an object whose `$type` names its collection and whose
/// every other key is an NSID.
impl TryFrom<AtprotoValue> for EntityBodyRecord {
	type Error = BevyError;
	fn try_from(value: AtprotoValue) -> Result<Self> {
		let Value::Map(map) = value.into_wire() else {
			bevybail!("an entity body record is an object");
		};
		let mut collection = None;
		let mut components = BTreeMap::new();
		for (key, value) in map.0 {
			if key == Self::TYPE_KEY {
				collection = Some(Nsid::parse(value.as_str()?)?);
				continue;
			}
			components
				.insert(Nsid::parse(&key)?, AtprotoValue::from_wire(value)?);
		}
		let Some(collection) = collection else {
			bevybail!("an entity body record names its collection in `$type`");
		};
		Self {
			collection,
			components,
		}
		.xok()
	}
}

/// A beet record: an entity body whose identity components derive its rkey,
/// `org.beet.*`. A scene body has no identity and is not one.
///
/// A collection's lexicon requires [`REQUIRED`](Self::REQUIRED), declares
/// [`OPTIONAL`](Self::OPTIONAL) and stores anything else untouched, a plugin's
/// component included. Serialized as the body alone (pair it with
/// `#[serde(try_from = "EntityBodyRecord", into = "EntityBodyRecord")]`),
/// reading through [`accept`](Self::accept), so [`rkey`](Self::rkey) is total
/// afterwards.
#[cfg(feature = "serde")]
pub trait EntityCollection: AtprotoRecord + Sized {
	/// The identity components; the rkey is a function of these alone.
	const REQUIRED: &'static [Nsid];
	/// Declared, not required: what the compiler may put here.
	const OPTIONAL: &'static [Nsid];

	/// The rkey `body` lives at, from its identity components.
	fn derive_rkey(body: &EntityBodyRecord) -> Result<Rkey>;

	/// The record of an accepted body and the rkey derived from it.
	fn from_parts(body: EntityBodyRecord, rkey: Rkey) -> Self;

	/// The body.
	fn body(&self) -> &EntityBodyRecord;

	/// The key this record lives at.
	fn rkey(&self) -> &Rkey;

	/// Refuse a body of another collection or missing a required component,
	/// so [`rkey`](Self::rkey) is total afterwards.
	fn accept(body: EntityBodyRecord) -> Result<Self> {
		if body.collection() != &Self::COLLECTION {
			bevybail!(
				"expected a `{}` record, found `{}`",
				Self::COLLECTION,
				body.collection()
			);
		}
		if let Some(missing) =
			Self::REQUIRED.iter().find(|nsid| !body.contains(nsid))
		{
			bevybail!("a `{}` record is missing `{missing}`", Self::COLLECTION);
		}
		let rkey = Self::derive_rkey(&body)?;
		Self::from_parts(body, rkey).xok()
	}
}

#[cfg(all(test, feature = "serde"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn writes_flat_and_sorted() {
		let mut body =
			EntityBodyRecord::new(nsid!("org.beet.entity"));
		body.insert_value(
			nsid!("com.example.plugin#zed"),
			value!({ "scale": 0.5 }).try_into().unwrap(),
		);
		body.insert(&Provenance {
			created: Timestamp::from_secs(0),
			..default()
		})
		.unwrap();
		AtprotoValue::from(body.clone())
			.as_map()
			.unwrap()
			.keys()
			.map(|key| key.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"$type".to_string(),
				"com.example.plugin#zed".into(),
				"org.beet.core#provenance".into(),
			]);
		// a plugin's component survives a binary that knows nothing of it
		EntityBodyRecord::try_from(AtprotoValue::from(body.clone()))
			.unwrap()
			.xpect_eq(body);
	}

	#[crate::test]
	fn refuses_what_names_no_collection() {
		EntityBodyRecord::try_from(
			AtprotoValue::try_from(value!({ "a.b.c": 1 })).unwrap(),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("$type");
	}
}
