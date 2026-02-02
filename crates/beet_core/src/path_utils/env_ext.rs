//! Cross-platform environment variable access.

use crate::prelude::*;
use thiserror::Error;

/// Error returned when an environment variable operation fails.
#[derive(Debug, Error)]
pub enum EnvError {
	/// The requested environment variable was not found.
	#[error("Environment variable not found: {0}")]
	NotFound(String),
}

/// Get the command line arguments, excluding the program name
pub fn args() -> Vec<String> {
	#[cfg(not(target_arch = "wasm32"))]
	return std::env::args().skip(1).collect();
	#[cfg(target_arch = "wasm32")]
	// Deno.args already excludes program name
	return array_ext::into_vec_str(js_runtime::env_args());
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


/// Get all environment variables
/// ## Panics
/// In wasm this will panic if `js_runtime::env_all` returns a malformed array
pub fn vars() -> Vec<(String, String)> {
	#[cfg(not(target_arch = "wasm32"))]
	{
		return std::env::vars().collect();
	}
	#[cfg(target_arch = "wasm32")]
	{
		// Enumerate via JS 2D array: Object.entries(Deno.env.toObject())
		use js_sys::Array;
		let entries = js_runtime::env_all();
		let mut out: Vec<(String, String)> = Vec::new();
		let len = entries.length();
		for i in 0..len {
			let pair = Array::from(&entries.get(i));
			let key = pair.get(0).as_string().unwrap();
			let value = pair.get(1).as_string().unwrap();
			out.push((key, value));
		}
		return out;
	}
}

/// Get all environment variables that match the given filter.
pub fn vars_filtered(filter: GlobFilter) -> Vec<(String, String)> {
	vars()
		.into_iter()
		.filter(|(key, _)| filter.passes(key))
		.collect()
}
