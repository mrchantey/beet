use http::StatusCode;
use std::fmt::Debug;
pub type Result<T> = std::result::Result<T, Error>;


/// Errors returned from a [`Request::fetch`]
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Request error: {0}")]
	ResponseNotOk(StatusCode),
	#[error("Network error: {0}")]
	NetworkError(String),
	#[error("Failed to serialize request: {0}")]
	Serialization(String),
	#[error("Failed to deserialize response: {0}")]
	Deserialization(String),
}

impl Error {
	pub fn network(msg: impl Debug) -> Self {
		Self::NetworkError(format!("{msg:?}"))
	}
	pub fn serialization(msg: impl Debug) -> Self {
		Self::Serialization(format!("{msg:?}"))
	}
	pub fn deserialization(msg: impl Debug) -> Self {
		Self::Deserialization(format!("{msg:?}"))
	}
}
