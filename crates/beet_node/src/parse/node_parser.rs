use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::futures_lite;
use bevy::tasks::futures_lite::Stream;


pub trait NodeParser {
	fn parse(
		&mut self,
		entity: AsyncEntity,
		bytes: impl AsRef<[u8]>,
	) -> impl Future<Output = Result> {
		let bytes = bytes.as_ref().to_vec();
		async move {
			self.parse_stream(entity, futures_lite::stream::once(Ok(bytes)))
				.await
		}
	}
	fn parse_stream(
		&mut self,
		entity: AsyncEntity,
		stream: impl 'static + Unpin + Stream<Item = Result<impl AsRef<[u8]>>>,
	) -> impl Future<Output = Result>;
}


#[derive(Debug, Default, Clone)]
pub struct PlainTextParser;

impl PlainTextParser {
	pub fn new() -> Self { Self::default() }
}

impl NodeParser for PlainTextParser {
	fn parse(
		&mut self,
		entity: AsyncEntity,
		bytes: impl AsRef<[u8]>,
	) -> impl Future<Output = Result> {
		async move {
			let text = std::str::from_utf8(bytes.as_ref())?;
			entity.insert_then(Value::new(text.to_string())).await;
			Ok(())
		}
	}

	fn parse_stream(
		&mut self,
		entity: AsyncEntity,
		stream: impl 'static + Unpin + Stream<Item = Result<impl AsRef<[u8]>>>,
	) -> impl Future<Output = Result> {
		let mut stream = stream_ext::bytes_to_text(stream);
		async move {
			entity.insert_then(Value:: new(String::new())).await;
			while let Some(result) = stream.next().await {
				let text = result?;
				entity
					.get_mut::<Value, _>(move |mut value| {
						match value.as_mut() {
							Value::Str(existing) => {
								existing.push_str(&text);
								Ok(())
							}
							_ => bevybail!("Expected a String Value"),
						}
					})
					.await??;
			}

			Ok(())
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::tasks::futures_lite::stream;

	/// Read a plain string and collect the result via [`PlainTextParser`].
	#[beet_core::test]
	async fn read_plain_text() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				PlainTextParser::default()
					.parse(entity, b"hello world")
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("hello world".into()));
	}

	/// Multiple chunks via `read` are concatenated in order.
	#[beet_core::test]
	async fn read_stream_simple_chunks() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let chunks: Vec<Result<Vec<u8>>> = vec![
					Ok(b"foo".to_vec()),
					Ok(b"bar".to_vec()),
					Ok(b"baz".to_vec()),
				];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks))
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("foobarbaz".into()));
	}

	/// A multi-byte UTF-8 character (4-byte emoji) split across chunk boundaries
	/// is reassembled losslessly.
	#[beet_core::test]
	async fn read_stream_utf8_split_across_chunks() {
		// 🌱 is U+1F331, encoded as the 4 bytes [0xF0, 0x9F, 0x8C, 0xB1]
		let seedling: [u8; 4] = [0xF0, 0x9F, 0x8C, 0xB1];
		AsyncPlugin::world()
			.run_async_local_then(move |world| async move {
				let entity = world.spawn_then(()).await;
				// Split the 4-byte sequence across three separate chunks
				let chunks: Vec<Result<Vec<u8>>> = vec![
					Ok(vec![seedling[0]]),
					Ok(vec![seedling[1], seedling[2]]),
					Ok(vec![seedling[3]]),
				];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks))
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("🌱".into()));
	}

	/// Valid UTF-8 before a split boundary is flushed immediately; the
	/// incomplete tail is carried to the next chunk.
	#[beet_core::test]
	async fn read_stream_valid_prefix_flushed_before_split() {
		// "hi" followed by the first byte of a 2-byte sequence (é = [0xC3, 0xA9])
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let chunks: Vec<Result<Vec<u8>>> = vec![
					Ok(vec![b'h', b'i', 0xC3]), // "hi" + incomplete é
					Ok(vec![0xA9, b'!']),       // completes é + "!"
				];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks))
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("hié!".into()));
	}

	/// An entirely incomplete UTF-8 sequence at the end of the stream is
	/// silently dropped (no panic or error).
	#[beet_core::test]
	async fn read_stream_truncated_utf8_at_eof() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// Only the first byte of a 2-byte sequence, nothing follows
				let chunks: Vec<Result<Vec<u8>>> =
					vec![Ok(b"ok".to_vec()), Ok(vec![0xC3])];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks))
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			// The valid "ok" prefix must be present; the dangling byte is discarded
			.xpect_eq(Value::Str("ok".into()));
	}

	/// An empty stream results in an empty string value being initialised.
	#[beet_core::test]
	async fn read_stream_empty() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let chunks: Vec<Result<Vec<u8>>> = vec![];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks))
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str(String::new()));
	}

	/// A stream error mid-flight propagates as an `Err`.
	#[beet_core::test]
	async fn read_stream_propagates_error() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let chunks: Vec<Result<Vec<u8>>> = vec![
					Ok(b"good".to_vec()),
					Err(bevyhow!("stream error")),
					Ok(b"never".to_vec()),
				];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks))
					.await
			})
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("stream error");
	}
}
