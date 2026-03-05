//! Stream conversion utilities for transforming byte streams into text streams.
//!
//! The primary entry point is [`bytes_to_text`], which handles buffering
//! of incomplete UTF-8 sequences across chunk boundaries.

use crate::prelude::*;
use futures_lite::Stream;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// A stream of [`Result<String>`] chunks decoded from a byte stream.
///
/// Created by [`bytes_to_text`]. Handles incomplete UTF-8 sequences that
/// span chunk boundaries by buffering partial bytes until enough data
/// arrives to decode them.
pub struct TextStream {
	inner: Pin<Box<dyn Stream<Item = Result<String, BevyError>>>>,
}

impl Stream for TextStream {
	type Item = Result<String, BevyError>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		self.get_mut().inner.as_mut().poll_next(cx)
	}
}

/// Attempt to decode as much valid UTF-8 as possible from `buffer`,
/// draining the decoded prefix. Returns `None` if the buffer is empty
/// or starts with an incomplete multibyte sequence.
fn drain_valid_utf8(buffer: &mut Vec<u8>) -> Option<String> {
	if buffer.is_empty() {
		return None;
	}
	match std::str::from_utf8(buffer) {
		Ok(text) => {
			let owned = text.to_string();
			buffer.clear();
			if owned.is_empty() { None } else { Some(owned) }
		}
		Err(err) => {
			let valid_up_to = err.valid_up_to();
			if valid_up_to > 0 {
				// Safety: validated by from_utf8
				let text = std::str::from_utf8(&buffer[..valid_up_to]).unwrap();
				let owned = text.to_string();
				buffer.drain(..valid_up_to);
				Some(owned)
			} else {
				None
			}
		}
	}
}

/// State carried between iterations of the unfold loop.
struct DecodeState<S> {
	stream: S,
	buffer: Vec<u8>,
	done: bool,
}

/// Convert a stream of byte chunks into a [`TextStream`] of UTF-8 string
/// chunks.
///
/// Bytes that form incomplete UTF-8 sequences at chunk boundaries are
/// buffered until the next chunk completes them. Any remaining bytes
/// after the source stream ends that cannot form valid UTF-8 are
/// reported as an error.
///
/// # Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_core::utils::stream_ext;
/// # async fn demo() -> Result {
/// let byte_stream = futures_lite::stream::once(Ok(b"hello".as_slice()));
/// let mut text_stream = stream_ext::bytes_to_text(byte_stream);
/// while let Some(result) = text_stream.next().await {
///     let text = result?;
///     assert_eq!(text, "hello");
/// }
/// # Ok(())
/// # }
/// ```
pub fn bytes_to_text(
	stream: impl 'static
	+ Stream<Item = Result<impl AsRef<[u8]>, BevyError>>
	+ Unpin,
) -> TextStream {
	let state = DecodeState {
		stream,
		buffer: Vec::new(),
		done: false,
	};

	let inner = futures_lite::stream::unfold(state, |mut state| async move {
		if state.done {
			// Flush remaining buffer after the source stream ended.
			if state.buffer.is_empty() {
				return None;
			}
			return match String::from_utf8(std::mem::take(&mut state.buffer)) {
				Ok(text) if text.is_empty() => None,
				Ok(text) => Some((Ok(text), state)),
				Err(err) => Some((Err(BevyError::from(err)), state)),
			};
		}

		loop {
			match state.stream.next().await {
				Some(Ok(data)) => {
					state.buffer.extend_from_slice(data.as_ref());
					if let Some(text) = drain_valid_utf8(&mut state.buffer) {
						return Some((Ok(text), state));
					}
					// Incomplete sequence, continue reading
				}
				Some(Err(err)) => {
					state.done = true;
					return Some((Err(err), state));
				}
				None => {
					state.done = true;
					if state.buffer.is_empty() {
						return None;
					}
					return match String::from_utf8(std::mem::take(
						&mut state.buffer,
					)) {
						Ok(text) if text.is_empty() => None,
						Ok(text) => Some((Ok(text), state)),
						Err(err) => Some((Err(BevyError::from(err)), state)),
					};
				}
			}
		}
	});

	TextStream {
		inner: Box::pin(inner),
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::utils::stream_ext;
	use futures_lite::stream;

	#[crate::test]
	async fn single_chunk() {
		let byte_stream =
			stream::once(Ok::<_, BevyError>(b"hello world".as_slice()));
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		text_stream
			.next()
			.await
			.unwrap()
			.unwrap()
			.xpect_eq("hello world");
		text_stream.next().await.xpect_none();
	}

	#[crate::test]
	async fn multiple_chunks() {
		let byte_stream = stream::iter(vec![
			Ok::<_, BevyError>(b"hello ".as_slice()),
			Ok(b"world".as_slice()),
		]);
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		text_stream
			.next()
			.await
			.unwrap()
			.unwrap()
			.xpect_eq("hello ");
		text_stream.next().await.unwrap().unwrap().xpect_eq("world");
		text_stream.next().await.xpect_none();
	}

	#[crate::test]
	async fn split_multibyte_utf8() {
		// '€' is encoded as [0xE2, 0x82, 0xAC] in UTF-8.
		// Split across two chunks to test buffering.
		let byte_stream = stream::iter(vec![
			Ok::<_, BevyError>(vec![b'a', 0xE2]),
			Ok(vec![0x82, 0xAC, b'b']),
		]);
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		// First chunk yields the valid 'a', buffers 0xE2
		text_stream.next().await.unwrap().unwrap().xpect_eq("a");
		// Second chunk completes '€' and yields '€b'
		text_stream.next().await.unwrap().unwrap().xpect_eq("€b");
		text_stream.next().await.xpect_none();
	}

	#[crate::test]
	async fn split_across_three_chunks() {
		// '€' = [0xE2, 0x82, 0xAC], split one byte per chunk
		let byte_stream = stream::iter(vec![
			Ok::<_, BevyError>(vec![0xE2]),
			Ok(vec![0x82]),
			Ok(vec![0xAC]),
		]);
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		// First two chunks buffer incomplete bytes, third completes '€'
		text_stream.next().await.unwrap().unwrap().xpect_eq("€");
		text_stream.next().await.xpect_none();
	}

	#[crate::test]
	async fn empty_chunks_skipped() {
		let byte_stream = stream::iter(vec![
			Ok::<_, BevyError>(b"".as_slice()),
			Ok(b"hi".as_slice()),
			Ok(b"".as_slice()),
		]);
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		text_stream.next().await.unwrap().unwrap().xpect_eq("hi");
		text_stream.next().await.xpect_none();
	}

	#[crate::test]
	async fn error_propagated() {
		let byte_stream = stream::iter(vec![
			Ok::<&[u8], BevyError>(b"ok".as_slice()),
			Err(bevyhow!("stream failed")),
			Ok(b"ignored".as_slice()),
		]);
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		text_stream.next().await.unwrap().unwrap().xpect_eq("ok");
		text_stream
			.next()
			.await
			.unwrap()
			.unwrap_err()
			.to_string()
			.xpect_contains("stream failed");
	}

	#[crate::test]
	async fn incomplete_utf8_at_end_is_error() {
		// Send an incomplete multibyte sequence with no follow-up
		let byte_stream = stream::once(Ok::<_, BevyError>(vec![0xE2, 0x82]));
		let mut text_stream = stream_ext::bytes_to_text(byte_stream);

		// The flush should produce an error for invalid UTF-8
		text_stream.next().await.unwrap().unwrap_err();
	}
}
