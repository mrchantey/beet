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
use core::pin::Pin;
use futures_core::Stream;
#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;

cfg_if! {
	if #[cfg(target_arch = "wasm32")] {
		/// Trait for streams implementing Send on non-wasm platforms.
		pub trait MaybeSendStream: Stream<Item = Result<Bytes>> + 'static {}
		impl<T> MaybeSendStream for T where T: Stream<Item = Result<Bytes>> + 'static {}
	} else {
		/// Trait for streams implementing Send on non-wasm platforms.
		pub trait MaybeSendStream:
			Stream<Item = Result<Bytes>> + Send + Sync + 'static
		{
		}
		impl<T> MaybeSendStream for T where
			T: Stream<Item = Result<Bytes>> + Send + Sync + 'static
		{
		}
	}
}

cfg_if! {
	if #[cfg(target_arch = "wasm32")] {
		/// Streaming body inner type. On wasm, wrapped in [`SendWrapper`] since
		/// streams may not be `Send + Sync`. On native, streams satisfy
		/// `Send + Sync` directly via [`MaybeSendStream`].
		type BodyStream = SendWrapper<Pin<Box<dyn MaybeSendStream>>>;
	} else {
		/// Streaming body inner type. On native platforms, streams are already
		/// `Send + Sync` via [`MaybeSendStream`].
		type BodyStream = Pin<Box<dyn MaybeSendStream>>;
	}
}

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
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				Body::Stream(SendWrapper::new(Box::pin(stream)))
			} else {
				Body::Stream(Box::pin(stream))
			}
		}
	}

	/// Converts this body into a [`TextStream`] of UTF-8 string chunks.
	#[cfg(feature = "std")]
	pub fn into_text_stream(self) -> TextStream {
		stream_ext::bytes_to_text(self)
	}
}

impl Default for Body {
	fn default() -> Self { Body::Bytes(Bytes::new()) }
}

impl core::fmt::Debug for Body {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
		cx: &mut core::task::Context<'_>,
	) -> core::task::Poll<Option<Self::Item>> {
		match &mut *self {
			Body::Bytes(bytes) => {
				if !bytes.is_empty() {
					let taken = core::mem::take(bytes);
					core::task::Poll::Ready(Some(Ok(taken)))
				} else {
					core::task::Poll::Ready(None)
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

	/// Creates a body by serializing `value` as JSON.
	#[cfg(feature = "json")]
	pub fn from_json<T: serde::Serialize>(value: &T) -> Result<Self> {
		MediaType::Json
			.serialize(value)
			.map(|bytes| Body::Bytes(Bytes::from(bytes)))
	}

	/// Creates a body from [`MediaBytes`], using the raw bytes.
	pub fn from_media(bytes: MediaBytes) -> Self {
		let (_media_type, bytes) = bytes.take();
		Body::Bytes(Bytes::from(bytes))
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

	/// Consumes the body and returns it as [`MediaBytes`], tagging the bytes with
	/// `content_type` (defaulting to [`MediaType::Bytes`] when `None`).
	///
	/// The shared body of [`Request::into_media_bytes`](crate::prelude::Request::into_media_bytes)
	/// and [`Response::into_media_bytes`](crate::prelude::Response::into_media_bytes),
	/// which each read their own `content-type` header and delegate here.
	pub async fn into_media_bytes(
		self,
		content_type: Option<MediaType>,
	) -> Result<MediaBytes> {
		let media_type = content_type.unwrap_or(MediaType::Bytes);
		let bytes = self.into_bytes().await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Consumes the body and decodes it into a [`Value`], a string or bytes per
	/// `content_type` (or UTF-8 validity when `None`).
	///
	/// A declared text media type decodes (lossily) as a [`Value::Str`]; a declared
	/// non-text type stays [`Value::Bytes`]; with no type the bytes are a string if
	/// valid UTF-8, else bytes. Uses beet's own [`Value`], so it is not json-gated.
	/// The shared body of [`Request::into_value`](crate::prelude::Request::into_value)
	/// and [`Response::into_value`](crate::prelude::Response::into_value).
	pub async fn into_value(
		self,
		content_type: Option<MediaType>,
	) -> Result<Value> {
		let bytes = self.into_bytes().await?;
		match content_type {
			// a declared text type is decoded as UTF-8 (lossily, never failing).
			Some(media_type) if media_type.is_text() => {
				Value::str(String::from_utf8_lossy(&bytes).into_owned())
			}
			// a declared non-text type stays bytes.
			Some(_) => Value::Bytes(bytes.to_vec()),
			// no type: a string if valid UTF-8, else bytes.
			None => match String::from_utf8(bytes.to_vec()) {
				Ok(text) => Value::str(text),
				Err(err) => Value::Bytes(err.into_bytes()),
			},
		}
		.xok()
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
				Ok(Some(core::mem::take(bytes)))
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
