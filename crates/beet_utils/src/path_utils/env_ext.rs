use thiserror::Error;

use crate::prelude::GlobFilter;

#[derive(Debug, Error)]
pub enum EnvError {
	#[error("Environment variable not found: {0}")]
	NotFound(String),
}

pub fn get(key: &str) -> Result<String, EnvError> {
	std::env::var(key).map_err(|_| EnvError::NotFound(key.to_string()))
}


/// Get all environment variables that match the given filter.
pub fn filtered(filter: GlobFilter) -> Vec<(String, String)> {
	std::env::vars()
		.filter(|(key, _)| filter.passes(key))
		.collect()
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get() {
		assert!(get("PATH").is_ok());
	}

	#[test]
	fn test_filtered() {
		let filter = GlobFilter::default().with_include("PATH");
		let vars = filtered(filter);
		assert_eq!(vars.len(), 1);
	}
}
