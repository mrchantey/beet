use crate::prelude::*;

/// A member of an open union: an object naming its own lexicon in `$type`,
/// carried whole, so a format this build does not know still reads and
/// writes back unchanged.
///
/// A lexicon field typed `union` without `closed: true` (a standard site
/// document's `content`, a post's `embed`) holds any object whose `$type` a
/// reader may or may not know. A reader that knows the type reads
/// [`fields`](Self::fields) as it; every other reader ignores it, which is the
/// point of an open union.
///
/// ```
/// # use beet_core::prelude::*;
/// let member = OpenUnion::new(
/// 	Nsid::new_static("app.bsky.embed.external"),
/// 	value!({ "uri": "https://beet.org" }),
/// )
/// .unwrap();
/// member.r#type().as_str().xpect_eq("app.bsky.embed.external");
/// member.fields().get("uri").unwrap().as_str().unwrap().xpect_eq("https://beet.org");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "Value", into = "Value"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct OpenUnion {
	/// The def the object implements.
	r#type: Nsid,
	/// Every other field of the object.
	fields: Map,
}

impl OpenUnion {
	/// The key a union member names its def under.
	pub const TYPE_KEY: &'static str = "$type";

	/// The member of def `r#type` holding `fields`, an object. A `$type`
	/// among the fields must name the same def.
	pub fn new(r#type: Nsid, fields: Value) -> Result<Self> {
		let Value::Map(mut fields) = fields else {
			bevybail!(
				"a `{}` union member must be an object, found {}",
				r#type,
				fields.kind()
			);
		};
		match fields.remove(Self::TYPE_KEY) {
			None => {}
			Some(Value::Str(named)) if named.as_str() == r#type.as_str() => {}
			Some(other) => bevybail!(
				"a `{}` union member declares `$type: {other}`",
				r#type
			),
		}
		Self { r#type, fields }.xok()
	}

	/// A serializable `value` as the member of def `r#type`.
	#[cfg(feature = "serde")]
	pub fn from_serde<T: Serialize>(r#type: Nsid, value: &T) -> Result<Self> {
		Self::new(r#type, Value::from_serde(value)?)
	}

	/// The def the object implements, ie `pub.leaflet.content`.
	pub fn r#type(&self) -> &Nsid { &self.r#type }

	/// The object's fields, its `$type` excluded.
	pub fn fields(&self) -> &Map { &self.fields }

	/// The fields read as a `T`, the type a reader knows this def as.
	#[cfg(feature = "serde")]
	pub fn into_serde<T: DeserializeOwned>(self) -> Result<T> {
		Value::Map(self.fields).into_serde()
	}
}

/// The object a member is written as, `$type` first.
impl From<OpenUnion> for Value {
	fn from(member: OpenUnion) -> Self {
		let mut map = Map::default();
		map.insert(OpenUnion::TYPE_KEY, member.r#type.as_str());
		map.extend(member.fields.0);
		Value::Map(map)
	}
}

/// An object read as a member, refused when it names no valid `$type`.
impl TryFrom<Value> for OpenUnion {
	type Error = BevyError;
	fn try_from(value: Value) -> Result<Self> {
		let r#type = value
			.as_map()
			.ok()
			.and_then(|map| map.get(Self::TYPE_KEY).ok())
			.and_then(|r#type| r#type.as_str().ok())
			.ok_or_else(|| {
				bevyhow!("a union member must be an object naming its `$type`")
			})?
			.xmap(Nsid::parse)?;
		Self::new(r#type, value)
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn round_trips_a_member() {
		let json = r#"{"$type":"app.bsky.embed.external","external":{"uri":"https://beet.org"}}"#;
		let member = serde_json::from_str::<OpenUnion>(json).unwrap();
		member.r#type().as_str().xpect_eq("app.bsky.embed.external");
		member.fields().contains("$type").xpect_false();
		serde_json::to_string(&member).unwrap().xpect_eq(json);
	}

	#[crate::test]
	fn refuses_a_member_without_a_type() {
		serde_json::from_str::<OpenUnion>(r#"{"uri":"https://beet.org"}"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("$type");
		OpenUnion::new(
			Nsid::new_static("com.example.a"),
			value!({ "$type": "com.example.b" }),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("com.example.b");
	}
}
