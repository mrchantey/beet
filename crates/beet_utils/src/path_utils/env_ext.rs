#[cfg(target_arch = "wasm32")]
use crate::js_runtime;
use crate::prelude::GlobFilter;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvError {
	#[error("Environment variable not found: {0}")]
	NotFound(String),
}

/// Try get the environment variable with the given key, returning
/// an error containing the key name if not found.
pub fn var(key: &str) -> Result<String, EnvError> {
	#[cfg(not(target_arch = "wasm32"))]
	{
		return std::env::var(key)
			.map_err(|_| EnvError::NotFound(key.to_string()));
	}

	#[cfg(target_arch = "wasm32")]
	{
		return js_runtime::env_var(key)
			.ok_or_else(|| EnvError::NotFound(key.to_string()));
	}
}

/// Get all environment variables that match the given filter.
pub fn vars_filtered(filter: GlobFilter) -> Vec<(String, String)> {
	#[cfg(not(target_arch = "wasm32"))]
	{
		return std::env::vars()
			.filter(|(key, _)| filter.passes(key))
			.collect();
	}

	#[cfg(all(target_arch = "wasm32", feature = "serde"))]
	{
		use std::collections::HashMap;
		use std::sync::OnceLock;
		static ENV_CACHE: OnceLock<HashMap<String, String>> = OnceLock::new();

		fn all() -> &'static HashMap<String, String> {
			ENV_CACHE.get_or_init(|| {
				let json = js_runtime::env_all_json();
				serde_json::from_str(&json).unwrap_or_default()
			})
		}

		return all()
			.iter()
			.filter(|(k, _)| filter.passes(k))
			.map(|(k, v)| (k.clone(), v.clone()))
			.collect();
	}

	// If on wasm without serde support, we can't list all envs; return empty.
	#[cfg(all(target_arch = "wasm32", not(feature = "serde")))]
	{
		return Vec::new();
	}
}

//upstream of sweet, see sweet/tests/env_ext for tests
