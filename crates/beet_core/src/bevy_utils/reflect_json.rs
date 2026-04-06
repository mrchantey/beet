//! Reflectable newtype wrappers for `serde_json` types.
//!
//! Because of Rust's orphan rules, we cannot implement `Reflect` directly
//! for foreign types like [`serde_json::Value`]. Instead we provide
//! transparent newtype wrappers that derive `Reflect` with
//! `#[reflect(opaque)]` and delegate to the inner type via `Deref`.
use crate::prelude::*;

/// Reflectable wrapper for [`serde_json::Value`].
#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	Default,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
	Reflect,
)]
#[reflect(opaque)]
#[reflect(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct JsonValue(pub serde_json::Value);

impl From<serde_json::Value> for JsonValue {
	fn from(value: serde_json::Value) -> Self { Self(value) }
}

impl From<JsonValue> for serde_json::Value {
	fn from(value: JsonValue) -> Self { value.0 }
}

/// Reflectable wrapper for [`serde_json::Map<String, serde_json::Value>`].
#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	Hash,
	Default,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
	Reflect,
)]
#[reflect(opaque)]
#[reflect(Clone, Debug, Hash, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct JsonMap(pub serde_json::Map<String, serde_json::Value>);

impl From<serde_json::Map<String, serde_json::Value>> for JsonMap {
	fn from(map: serde_json::Map<String, serde_json::Value>) -> Self {
		Self(map)
	}
}

impl From<JsonMap> for serde_json::Map<String, serde_json::Value> {
	fn from(map: JsonMap) -> Self { map.0 }
}
