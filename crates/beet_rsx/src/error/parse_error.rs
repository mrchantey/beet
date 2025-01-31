#[cfg(not(target_arch = "wasm32"))]
use sweet::prelude::FsError;
use thiserror::Error;

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, Error)]
pub enum ParseError {
	#[cfg(not(target_arch = "wasm32"))]
	#[error("{0}")]
	Fs(FsError),
	#[error("Hydration Error: {0}")]
	Hydration(String),
	#[error("Serde Error: {0}")]
	Serde(String),
	#[error("Parse Error: {0}")]
	Other(String),
}
impl ParseError {
	pub fn serde(e: impl AsRef<str>) -> Self {
		Self::Serde(e.as_ref().to_string())
	}

	pub fn hydration(
		expected: impl AsRef<str>,
		receieved: impl AsRef<str>,
	) -> Self {
		Self::Hydration(format!(
			"Expected: {}\nReceieved: {}",
			expected.as_ref(),
			receieved.as_ref()
		))
	}
}


#[cfg(not(target_arch = "wasm32"))]
impl From<FsError> for ParseError {
	fn from(e: FsError) -> Self { Self::Fs(e) }
}

impl From<anyhow::Error> for ParseError {
	fn from(e: anyhow::Error) -> Self { Self::Other(e.to_string()) }
}
impl From<String> for ParseError {
	fn from(e: String) -> Self { Self::Other(e) }
}
impl From<&str> for ParseError {
	fn from(e: &str) -> Self { Self::Other(e.to_string()) }
}

impl From<std::num::ParseIntError> for ParseError {
	fn from(e: std::num::ParseIntError) -> Self { Self::Other(e.to_string()) }
}
