//! HTTP body types for request and response handling.
//!
//! This module provides the [`Body`] type, which represents the body of an
//! HTTP request or response. It supports both in-memory bytes and streaming
//! content.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! // Create from bytes
//! let body: Body = "Hello, world!".into();
//!
//! // Create from a stream
//! let stream = futures::stream::once(async { Ok(bytes::Bytes::from("data")) });
//! let body = Body::stream(stream);
//! ```

use beet_core::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use bytes::Bytes;
use futures::Stream;
#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;
use std::pin::Pin;

#[cfg(target_arch = "wasm32")]
/// Trait for streams implementing Send on non-wasm platforms
pub trait MaybeSendStream: Stream<Item = Result<Bytes>> + 'static {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSendStream for T where T: Stream<Item = Result<Bytes>> + 'static {}

#[cfg(not(target_arch = "wasm32"))]
/// Trait for streams implementing Send on non-wasm platforms
pub trait MaybeSendStream:
	Stream<Item = Result<Bytes>> + Send + Sync + 'static
{
}
#[cfg(not(target_arch = "wasm32"))]
impl<T> MaybeSendStream for T where
	T: Stream<Item = Result<Bytes>> + Send + Sync + 'static
{
}

/// Streaming body inner type. On wasm, wrapped in [`SendWrapper`] since
/// streams may not be `Send + Sync`. On native, streams satisfy
/// `Send + Sync` directly via [`MaybeSendStream`].
#[cfg(target_arch = "wasm32")]
type BodyStream = SendWrapper<Pin<Box<dyn MaybeSendStream>>>;
/// Streaming body inner type. On native platforms, streams are already
/// `Send + Sync` via [`MaybeSendStream`].
#[cfg(not(target_arch = "wasm32"))]
type BodyStream = Pin<Box<dyn MaybeSendStream>>;

/// The body of an HTTP request or response.
///
/// Bodies can be either in-memory [`Bytes`] or a streaming source.
/// The type implements [`Stream`] for async iteration.
pub enum Body {
	/// In-memory bytes content.
	Bytes(Bytes),
	/// A streaming body source.
	Stream(BodyStream),
}

impl Body {
	/// Creates a streaming body from the given stream.
	pub fn stream(stream: impl 'static + MaybeSendStream) -> Self {
		#[cfg(target_arch = "wasm32")]
		{
			Body::Stream(SendWrapper::new(Box::pin(stream)))
		}
		#[cfg(not(target_arch = "wasm32"))]
		{
			Body::Stream(Box::pin(stream))
		}
	}

	/// Converts this body into a [`TextStream`] of UTF-8 string chunks.
	pub fn into_text_stream(self) -> TextStream {
		stream_ext::bytes_to_text(self)
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
	#[cfg(feature = "json")]
	pub async fn into_json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		let bytes = self.into_bytes().await?;
		MediaType::Json.deserialize(&bytes)
	}

	/// Consumes the body and deserializes the content as postcard.
	#[cfg(feature = "postcard")]
	pub async fn into_postcard<T: serde::de::DeserializeOwned>(
		self,
	) -> Result<T> {
		let bytes = self.into_bytes().await?;
		MediaType::Postcard.deserialize(&bytes)
	}

	/// Creates a body by serializing `value` as JSON.
	#[cfg(feature = "json")]
	pub fn from_json<T: serde::Serialize>(value: &T) -> Result<Self> {
		MediaType::Json
			.serialize(value)
			.map(|bytes| Body::Bytes(Bytes::from(bytes)))
	}

	/// Creates a body by serializing `value` as postcard.
	#[cfg(feature = "postcard")]
	pub fn from_postcard<T: serde::Serialize>(value: &T) -> Result<Self> {
		MediaType::Postcard
			.serialize(value)
			.map(|bytes| Body::Bytes(Bytes::from(bytes)))
	}

	/// Consumes the body and deserializes using the given [`MediaType`].
	#[cfg(feature = "serde")]
	pub async fn into_media_type<T: serde::de::DeserializeOwned>(
		self,
		media_type: MediaType,
	) -> Result<T> {
		let bytes = self.into_bytes().await?;
		media_type.deserialize(&bytes)
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

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	async fn into_text_stream_from_bytes() {
		let body: Body = "hello world".into();
		let mut stream = body.into_text_stream();

		stream
			.next()
			.await
			.unwrap()
			.unwrap()
			.xpect_eq("hello world");
		stream.next().await.xpect_none();
	}

	#[beet_core::test]
	async fn into_text_stream_from_stream() {
		let byte_stream = futures::stream::iter(vec![
			Ok(Bytes::from("hello ")),
			Ok(Bytes::from("world")),
		]);
		let body = Body::stream(byte_stream);
		let mut stream = body.into_text_stream();

		stream.next().await.unwrap().unwrap().xpect_eq("hello ");
		stream.next().await.unwrap().unwrap().xpect_eq("world");
		stream.next().await.xpect_none();
	}

	#[beet_core::test]
	async fn into_text_stream_multibyte_split() {
		// '€' = [0xE2, 0x82, 0xAC], split across two chunks
		let byte_stream = futures::stream::iter(vec![
			Ok(Bytes::from_static(&[b'a', 0xE2])),
			Ok(Bytes::from_static(&[0x82, 0xAC, b'b'])),
		]);
		let body = Body::stream(byte_stream);
		let mut stream = body.into_text_stream();

		stream.next().await.unwrap().unwrap().xpect_eq("a");
		stream.next().await.unwrap().unwrap().xpect_eq("€b");
		stream.next().await.xpect_none();
	}

	#[beet_core::test]
	async fn into_text_stream_empty_body() {
		let body = Body::default();
		let mut stream = body.into_text_stream();
		stream.next().await.xpect_none();
	}
}
