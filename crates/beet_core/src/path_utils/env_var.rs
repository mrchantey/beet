//! Environment variable wrapper with serde and reflect support.

use crate::prelude::*;

/// Serde and reflect-friendly environment variable wrapper.
///
/// Serializes only the environment variable key; resolves the value
/// from the current environment on deserialization.
///
/// ## Example
/// ```rust
/// # use beet_core::prelude::*;
/// // assumes MY_VAR is set
/// std::env::set_var("MY_VAR", "hello");
/// let var = EnvVar::new("MY_VAR").unwrap();
/// assert_eq!(var.key(), "MY_VAR");
/// assert_eq!(var.value(), "hello");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Deref, Get)]
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

	#[test]
	fn roundtrip_serde() {
		// SAFETY: single-threaded test
		unsafe { std::env::set_var("BEET_TEST_ENV_VAR", "test_value") };
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
