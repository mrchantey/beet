use thiserror::Error;

use crate::prelude::GlobFilter;

#[derive(Debug, Error)]
pub enum EnvError {
	#[error("Environment variable not found: {0}")]
	NotFound(String),
}

/// Try get the environment variable with the given key, returning
/// an error containing the key name if not found.
pub fn var(key: &str) -> Result<String, EnvError> {
	std::env::var(key).map_err(|_| EnvError::NotFound(key.to_string()))
}


/// Get all environment variables that match the given filter.
pub fn vars_filtered(filter: GlobFilter) -> Vec<(String, String)> {
	std::env::vars()
		.filter(|(key, _)| filter.passes(key))
		.collect()
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_var() {
		assert!(var("PATH").is_ok());
	}

	#[test]
	fn test_vars_filtered() {
		let filter = GlobFilter::default().with_include("PATH");
		let vars = vars_filtered(filter);
		assert_eq!(vars.len(), 1);
	}
}
