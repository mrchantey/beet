use beet_core::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use bytes::Bytes;
use futures::Stream;
use send_wrapper::SendWrapper;
use std::pin::Pin;


#[cfg(target_arch = "wasm32")]
type DynBytesStream = dyn Stream<Item = Result<Bytes>>;
/// crates like Axum require Stream to be Send
#[cfg(not(target_arch = "wasm32"))]
type DynBytesStream = dyn Stream<Item = Result<Bytes>> + Send + Sync;



pub enum Body {
	Bytes(Bytes),
	Stream(SendWrapper<Pin<Box<DynBytesStream>>>),
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

impl Into<Body> for Bytes {
	fn into(self) -> Body { Body::Bytes(self) }
}

impl Body {
	/// Any body with a content length greater than this will be parsed as a stream.
	pub const MAX_BUFFER_SIZE: usize = 1 * 1024 * 1024; // 1 MB

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

	pub async fn into_string(self) -> Result<String> {
		let bytes = self.into_bytes().await?;
		String::from_utf8(bytes.to_vec())?.xok()
	}

	// temp antipattern while migrating beet_router
	pub fn try_into_bytes(self) -> Option<Bytes> {
		match self {
			Body::Bytes(bytes) => Some(bytes),
			Body::Stream(_) => None,
		}
	}

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

	pub fn bytes_eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Body::Bytes(a), Body::Bytes(b)) => a == b,
			_ => false,
		}
	}
}
