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
//!   "resources": { "bevy_time::time::Time": 393933892 },
//!   "entities": {
//!     "0": { "bevy_ecs::name::Name": "Bob" },
//!     "1": { "bevy_ecs::hierarchy::ChildOf": "0" }
//!   }
//! }
//! ```
//!
//! Every value is a plain [`Value`], so a body is read and written with no
//! type registry: reflection happens on the way to and from a world, in
//! `template_serde` (`SceneBody::reflect` and `TemplateBuilder`).
use crate::prelude::*;
use indexmap::IndexMap;

/// A scene as data: its resources and its entities, each an [`EntityBody`] of
/// type path to [`Value`]. See the [module docs](self) for the wire shape.
///
/// The serde form of every scene format (json, ron, postcard), and
/// `org.beet.scene#body` as an atproto lexicon object ([`NSID`](Self::NSID)).
/// Two bodies are equal when they write the same document, so entity order
/// counts.
#[derive(Debug, Default, Clone, Eq, Serialize, Deserialize)]
pub struct SceneBody {
	/// The scene's resources, keyed by type path.
	#[serde(default)]
	pub resources: EntityBody,
	/// The scene's entities in file order, which is child order and is never
	/// sorted.
	#[serde(default)]
	pub entities: IndexMap<EntityKey, EntityBody, FixedHasher>,
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

	/// Read a body from a serde format's bytes: ron, json or postcard.
	pub fn from_bytes(bytes: &MediaBytes) -> Result<Self> {
		match bytes.media_type() {
			MediaType::Ron | MediaType::Json | MediaType::Postcard => {
				bytes.deserialize()
			}
			// untyped bytes are the binary format
			MediaType::Bytes => MediaType::Postcard.deserialize(bytes.bytes()),
			other => {
				bevybail!(
					"a scene body is ron, json or postcard, not `{other}`"
				)
			}
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
