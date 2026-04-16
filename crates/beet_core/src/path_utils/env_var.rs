//! Environment variable wrapper with serde and reflect support.

use crate::prelude::*;

/// Serde and reflect-friendly environment variable wrapper.
///
/// Serializes only the environment variable key; resolves the value
/// from the current environment on deserialization.
///
/// When used with bevy reflect serialization, registering the type also
/// registers [`bevy_reflect::ReflectSerialize`] and [`bevy_reflect::ReflectDeserialize`]
/// (requires the `serde` feature), so the reflect round-trip uses the custom
/// serde impls rather than the struct reflection path. This ensures `value`
/// is always resolved from the environment rather than defaulting to `""`.
///
/// ## Example
/// ```rust
/// # use beet_core::prelude::*;
/// // SAFETY: single-threaded example
/// unsafe { env_ext::set_var("MY_VAR", "hello") };
/// let var = EnvVar::new("MY_VAR").unwrap();
/// assert_eq!(var.key(), "MY_VAR");
/// assert_eq!(var.value(), "hello");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Deref, Get)]
// Disable auto-derived FromReflect (it would default `value` to "");
// we register ReflectFromReflect manually via the FromReflect ident.
#[reflect(from_reflect = false, FromReflect)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct EnvVar {
	/// The environment variable key name.
	key: String,
	/// The resolved value, not serialized.
	#[reflect(ignore)]
	#[deref]
	value: String,
}

impl EnvVar {
	/// Creates a new [`EnvVar`] by resolving `key` from the current environment.
	pub fn new(key: &str) -> Result<Self> {
		let value = env_ext::var(key)?;
		Ok(Self {
			key: key.to_string(),
			value,
		})
	}
}

/// Manual [`FromReflect`] implementation.
///
/// The auto-derived impl only looks at `reflect_ref()` struct fields, giving
/// `value = ""` for the ignored field. We fix this by:
/// 1. Trying a direct downcast first (succeeds when the value was returned
///    by [`bevy_reflect::ReflectDeserialize`], ie a concrete [`EnvVar`]).
/// 2. Falling back to the struct reflection path and calling [`EnvVar::new`]
///    to properly resolve `value` from the environment.
impl FromReflect for EnvVar {
	fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
		// Concrete value (returned by the ReflectDeserialize path)
		if let Some(value) = reflect.try_downcast_ref::<EnvVar>() {
			return Some(value.clone());
		}
		// Dynamic struct path: reconstruct by resolving value from the env
		if let bevy_reflect::ReflectRef::Struct(dyn_struct) =
			reflect.reflect_ref()
		{
			let key = <String as FromReflect>::from_reflect(
				dyn_struct.field("key")?,
			)?;
			EnvVar::new(&key).ok()
		} else {
			None
		}
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for EnvVar {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> std::result::Result<S::Ok, S::Error> {
		self.key.serialize(serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for EnvVar {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> std::result::Result<Self, D::Error> {
		let key = String::deserialize(deserializer)?;
		let value = env_ext::var(&key).map_err(serde::de::Error::custom)?;
		Ok(EnvVar { key, value })
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	// todo js runtime set env var
	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	fn roundtrip_serde() {
		// SAFETY: single-threaded test
		unsafe { env_ext::set_var("BEET_TEST_ENV_VAR", "test_value") };
		let var = EnvVar::new("BEET_TEST_ENV_VAR").unwrap();
		var.key().xpect_eq("BEET_TEST_ENV_VAR");
		var.value().xpect_eq("test_value");

		// serializes as just the key string
		let json = serde_json::to_string(&var).unwrap();
		json.xpect_eq("\"BEET_TEST_ENV_VAR\"");

		// deserializes by resolving the env var
		let roundtripped: EnvVar = serde_json::from_str(&json).unwrap();
		roundtripped.key().xpect_eq("BEET_TEST_ENV_VAR");
		roundtripped.value().xpect_eq("test_value");
	}

	#[test]
	fn deserialize_missing_var_errors() {
		serde_json::from_str::<EnvVar>("\"BEET_NONEXISTENT_VAR_XYZ\"")
			.unwrap_err();
	}
}

#[cfg(all(test, feature = "serde"))]
#[cfg(not(target_arch = "wasm32"))]
mod reflect_test {
	use super::*;
	use bevy_reflect::FromReflect;
	use bevy_reflect::TypeRegistry;
	use bevy_reflect::serde::ReflectDeserializer;
	use bevy_reflect::serde::ReflectSerializer;
	use serde::de::DeserializeSeed;

	/// Verifies that `EnvVar` survives a bevy reflect serde round-trip.
	///
	/// `ReflectSerializer` uses the registered `ReflectSerialize` type data
	/// (from `#[reflect(Serialize, Deserialize)]`) to output only the key
	/// string. `ReflectDeserializer` uses `ReflectDeserialize` to call our
	/// custom `serde::Deserialize` impl, which resolves `value` from the
	/// environment. The manual `FromReflect` impl then downcasts the
	/// concrete [`EnvVar`] directly rather than reconstructing it from
	/// struct fields (which would leave `value = ""`).
	#[test]
	fn roundtrip_reflect_serde() {
		// SAFETY: single-threaded test
		unsafe { env_ext::set_var("BEET_TEST_REFLECT_VAR", "reflect_value") };
		let var = EnvVar::new("BEET_TEST_REFLECT_VAR").unwrap();

		// Registering EnvVar also registers ReflectSerialize + ReflectDeserialize
		// via the #[reflect(Serialize, Deserialize)] attribute.
		let mut registry = TypeRegistry::default();
		registry.register::<EnvVar>();

		// Serialize: ReflectSerializer finds ReflectSerialize and calls our
		// serde::Serialize impl, outputting just the key string.
		let serializer =
			ReflectSerializer::new(var.as_partial_reflect(), &registry);
		let serialized = ron::to_string(&serializer).unwrap();

		// Deserialize: ReflectDeserializer finds ReflectDeserialize and calls
		// our serde::Deserialize impl, which resolves the value from the env.
		let reflect_deserializer = ReflectDeserializer::new(&registry);
		let deserialized = reflect_deserializer
			.deserialize(&mut ron::Deserializer::from_str(&serialized).unwrap())
			.unwrap();

		// from_reflect: downcasts the concrete EnvVar returned by
		// ReflectDeserialize rather than reconstructing from struct fields.
		let converted =
			<EnvVar as FromReflect>::from_reflect(&*deserialized).unwrap();
		converted.key().xpect_eq("BEET_TEST_REFLECT_VAR");
		converted.value().xpect_eq("reflect_value");
	}
}
