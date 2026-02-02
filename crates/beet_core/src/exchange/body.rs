//! HTTP body types for request and response handling.
//!
//! This module provides the [`Body`] type, which represents the body of an
//! HTTP request or response. It supports both in-memory bytes and streaming
//! content.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! // Create from bytes
//! let body: Body = "Hello, world!".into();
//!
//! // Create from a stream
//! let stream = futures::stream::once(async { Ok(bytes::Bytes::from("data")) });
//! let body = Body::stream(stream);
//! ```

use crate::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use bytes::Bytes;
use futures::Stream;
use send_wrapper::SendWrapper;
use std::pin::Pin;


#[cfg(target_arch = "wasm32")]
type DynBytesStream = dyn Stream<Item = Result<Bytes>>;
/// Crates like Axum require Stream to be Send.
// TODO we dont use axum anymore, can we make stream non-send?
#[cfg(not(target_arch = "wasm32"))]
type DynBytesStream = dyn Stream<Item = Result<Bytes>> + Send + Sync;


/// The body of an HTTP request or response.
///
/// Bodies can be either in-memory [`Bytes`] or a streaming source.
/// The type implements [`Stream`] for async iteration.
pub enum Body {
	/// In-memory bytes content.
	Bytes(Bytes),
	/// A streaming body wrapped in [`SendWrapper`] for use in Bevy components.
	Stream(SendWrapper<Pin<Box<DynBytesStream>>>),
}

impl Body {
	/// Creates a streaming body from the given stream.
	pub fn stream(
		stream: impl 'static + Stream<Item = Result<Bytes>> + Send + Sync,
	) -> Self {
		Body::Stream(SendWrapper::new(Box::pin(stream)))
	}
}

impl Default for Body {
	fn default() -> Self { Body::Bytes(Bytes::new()) }
}

impl std::fmt::Debug for Body {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Body::Bytes(bytes) => write!(f, "Body::Bytes({:?})", bytes),
			Body::Stream(_) => write!(f, "Body::Stream(...)"),
		}
	}
}

impl Stream for Body {
	type Item = Result<Bytes>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		match &mut *self {
			Body::Bytes(bytes) => {
				if !bytes.is_empty() {
					let taken = std::mem::take(bytes);
					std::task::Poll::Ready(Some(Ok(taken)))
				} else {
					std::task::Poll::Ready(None)
				}
			}
			Body::Stream(stream) => Pin::new(stream).poll_next(cx),
		}
	}
}


impl Into<Body> for &[u8] {
	fn into(self) -> Body { Body::Bytes(Bytes::from(self.to_vec())) }
}
impl Into<Body> for &str {
	fn into(self) -> Body { Body::Bytes(Bytes::from(self.as_bytes().to_vec())) }
}
impl Into<Body> for String {
	fn into(self) -> Body { Body::Bytes(Bytes::from(self.as_bytes().to_vec())) }
}
impl Into<Body> for Bytes {
	fn into(self) -> Body { Body::Bytes(self) }
}
impl Into<Body> for Vec<u8> {
	fn into(self) -> Body { Body::Bytes(Bytes::from(self)) }
}



impl Body {
	/// Maximum buffer size for parsing streaming bodies.
	///
	/// Any body with a content length greater than this will be parsed as a stream.
	pub const MAX_BUFFER_SIZE: usize = 1 * 1024 * 1024; // 1 MB

	/// Consumes the body and returns the full content as bytes.
	///
	/// For streaming bodies, this collects all chunks into a single buffer.
	pub async fn into_bytes(mut self) -> Result<Bytes> {
		match self {
			Body::Bytes(bytes) => Ok(bytes),
			Body::Stream(_) => {
				let mut buffer = bytes::BytesMut::new();
				while let Some(chunk) = self.next().await? {
					buffer.extend_from_slice(&chunk);
				}
				Ok(buffer.freeze())
			}
		}
	}

	/// Consumes the body and returns the content as a UTF-8 string.
	pub async fn into_string(self) -> Result<String> {
		let bytes = self.into_bytes().await?;
		String::from_utf8(bytes.to_vec())?.xok()
	}

	/// Consumes the body and deserializes the content as JSON.
	#[cfg(feature = "serde")]
	pub async fn into_json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		let bytes = self.into_bytes().await?;
		serde_json::from_slice::<T>(&bytes)
			.map_err(|e| bevyhow!("Failed to deserialize body\n {}", e))
	}

	/// Attempts to extract bytes without consuming a stream.
	///
	/// Returns `None` if this is a streaming body.
	// temp antipattern while migrating beet_router
	pub fn try_into_bytes(self) -> Option<Bytes> {
		match self {
			Body::Bytes(bytes) => Some(bytes),
			Body::Stream(_) => None,
		}
	}

	/// Returns the next chunk of data from the body.
	///
	/// For byte bodies, returns the entire content on the first call if not empty, then `None`.
	/// For streaming bodies, returns chunks as they become available.
	pub async fn next(&mut self) -> Result<Option<Bytes>> {
		match self {
			Body::Bytes(bytes) if !bytes.is_empty() => {
				Ok(Some(std::mem::take(bytes)))
			}
			Body::Bytes(_) => Ok(None),
			Body::Stream(stream) => match stream.next().await {
				Some(result) => Ok(Some(result?)),
				None => Ok(None),
			},
		}
	}

	/// Compares two bodies for byte equality.
	///
	/// Only returns `true` if both bodies are byte-based and contain the same data.
	/// Streaming bodies are never considered equal.
	pub fn bytes_eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Body::Bytes(a), Body::Bytes(b)) => a == b,
			_ => false,
		}
	}
}
