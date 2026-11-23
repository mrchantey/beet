//! [`SceneBody`]: a scene as data, the one shape every format writes.
//!
//! # Wire shape
//!
//! The document is flat at every level: a map of type path to value is a
//! resource set or an entity, and nothing wraps either. Entities are keyed by
//! [`EntityKey`] in file order, which is child order, and a reference to an
//! entity is its key.
//!
//! ```json
//! {
//!   "$type": "org.beet.scene#body",
//!   "resources": { "bevy_time::time::Time": 393933892 },
//!   "entities": {
//!     "0": { "bevy_ecs::name::Name": "Bob" },
//!     "1": { "bevy_ecs::hierarchy::ChildOf": "0" }
//!   }
//! }
//! ```
//!
//! A body is always written with its `$type`, which is how a structured
//! document says it is a scene body rather than a plain value: the structured
//! parsers in `template_serde` read one whose root names it as a body and any
//! other as one entity holding the value. Read back, the `$type` may be absent,
//! as it is in an atproto union member's fields, but never names another def.
//!
//! Every value is a plain [`Value`], so a body is read and written with no
//! type registry: reflection happens on the way to and from a world, in
//! `template_serde` (`SceneBody::reflect` and `TemplateBuilder`).
use crate::prelude::*;
use indexmap::IndexMap;

/// A scene as data: its resources and its entities, each an [`EntityBody`] of
/// type path to [`Value`]. See the [module docs](self) for the wire shape.
///
/// The serde form of every scene format (json, postcard, toml), and
/// `org.beet.scene#body` as an atproto lexicon object ([`NSID`](Self::NSID)).
/// Two bodies are equal when they write the same document, so entity order
/// counts.
#[derive(Debug, Default, Clone, Eq)]
pub struct SceneBody {
	/// The scene's resources, keyed by type path.
	pub resources: EntityBody,
	/// The scene's entities in file order, which is child order and is never
	/// sorted.
	pub entities: IndexMap<EntityKey, EntityBody, FixedHasher>,
}

/// One serde path for every format through the private [`Wire`] shape, a
/// map in a self describing format and positional fields in postcard, whose
/// leading `$type` doubles as the magic telling a body from a value.
impl Serialize for SceneBody {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		WireRef {
			r#type: SceneBodyType,
			resources: &self.resources,
			entities: &self.entities,
		}
		.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for SceneBody {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		let Wire {
			r#type: SceneBodyType,
			resources,
			entities,
		} = Wire::deserialize(deserializer)?;
		Ok(Self {
			resources,
			entities,
		})
	}
}

/// The body as written, `$type` first, so a reader probing a structured
/// document's root finds it before anything else. Borrowed, so a write
/// clones nothing.
#[derive(Serialize)]
struct WireRef<'a> {
	#[serde(rename = "$type")]
	r#type: SceneBodyType,
	resources: &'a EntityBody,
	entities: &'a IndexMap<EntityKey, EntityBody, FixedHasher>,
}

/// The body as read: its `$type` checked, and absent in a union member's
/// fields or a hand written document, as either half may be.
#[derive(Deserialize)]
struct Wire {
	#[serde(rename = "$type", default)]
	r#type: SceneBodyType,
	#[serde(default)]
	resources: EntityBody,
	#[serde(default)]
	entities: IndexMap<EntityKey, EntityBody, FixedHasher>,
}

/// A `$type` that must name [`SceneBody::NSID`], written as a plain string.
#[derive(Default)]
struct SceneBodyType;

impl Serialize for SceneBodyType {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(SceneBody::NSID.as_str())
	}
}

impl<'de> Deserialize<'de> for SceneBodyType {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		let named = SmolStr::deserialize(deserializer)?;
		if named.as_str() == SceneBody::NSID.as_str() {
			Ok(Self)
		} else {
			Err(serde::de::Error::custom(format!(
				"a scene body's `$type` is `{}`, not `{named}`",
				SceneBody::NSID
			)))
		}
	}
}

impl PartialEq for SceneBody {
	fn eq(&self, other: &Self) -> bool {
		self.resources == other.resources
			&& self.entities.iter().eq(other.entities.iter())
	}
}

impl SceneBody {
	/// The def a scene body is as an atproto lexicon object, the `$type` a
	/// structured document carries to say it is one.
	pub const NSID: Nsid = Nsid::new_static("org.beet.scene#body");

	/// Whether `value` is a body: an object whose `$type` names one.
	pub fn is_body(value: &Value) -> bool {
		value
			.as_map()
			.ok()
			.and_then(|map| map.get(OpenUnion::TYPE_KEY).ok())
			.and_then(|r#type| r#type.as_str().ok())
			.is_some_and(|r#type| r#type == Self::NSID.as_str())
	}

	/// A body of one entity, the root, holding `components`.
	pub fn root(components: EntityBody) -> Self {
		let mut entities = IndexMap::default();
		entities.insert(EntityKey::ROOT, components);
		Self {
			resources: default(),
			entities,
		}
	}

	/// Write this body in a serde format.
	pub fn to_bytes(&self, media_type: MediaType) -> Result<MediaBytes> {
		self.to_bytes_with_options(media_type, default())
	}

	/// Write this body in a serde format with [`SerializeOptions`].
	pub fn to_bytes_with_options(
		&self,
		media_type: MediaType,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		MediaBytes::serialize_with_options(media_type, self, options)
	}

	/// This body with its entities in the order a record holds them: by key,
	/// shortest first then bytewise.
	///
	/// The protocol's data model sorts the keys of every map, so entity order
	/// cannot ride a record the way it rides a file. As a lexicon object the
	/// order is therefore defined by the keys themselves, which for the keys a
	/// save mints (`"0"`, `"1"`, .. `"10"`) is file order, whatever order a
	/// transport delivered them in.
	pub fn in_record_order(mut self) -> Self {
		self.entities.sort_by(|left, _, right, _| {
			(left.as_str().len(), left.as_str())
				.cmp(&(right.as_str().len(), right.as_str()))
		});
		self
	}

	/// Read a body from the [`Value`] a document holds it as.
	pub fn from_value(value: Value) -> Result<Self> { value.into_serde() }

	/// This body as a [`Value`], its entities in file order: what a document
	/// holds, and what a record carries inline.
	pub fn to_value(&self) -> Result<Value> { Value::from_serde(self) }
}
