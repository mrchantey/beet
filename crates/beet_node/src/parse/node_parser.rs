use beet_core::prelude::*;
use bevy::tasks::futures_lite::Stream;
use bevy::tasks::futures_lite::StreamExt;


/// Parses bytes or a byte stream into an entity's components.
///
/// Implementors provide [`NodeParser::parse`]; the default [`NodeParser::parse_stream`]
/// collects the entire stream into a single buffer and delegates to it. This keeps
/// diffing, span tracking and component insertion logic in one place.
pub trait NodeParser {
	/// Parse a complete byte buffer and apply the result to `entity`.
	///
	/// An optional `path` enables [`FileSpan`] tracking on the produced nodes.
	fn parse(
		&mut self,
		entity: AsyncEntity,
		bytes: Vec<u8>,
		path: Option<WsPathBuf>,
	) -> impl Future<Output = Result>;

	/// ## Default Behavior
	/// Collect a byte stream to completion and then call [`NodeParser::parse`].
	/// ## Errors
	/// Propagates the first stream error encountered without calling `parse`.
	fn parse_stream(
		&mut self,
		entity: AsyncEntity,
		stream: impl 'static + Unpin + Stream<Item = Result<impl AsRef<[u8]>>>,
		path: Option<WsPathBuf>,
	) -> impl Future<Output = Result> {
		async move {
			let mut buf: Vec<u8> = Vec::new();
			let mut stream = stream;
			while let Some(result) = stream.next().await {
				buf.extend_from_slice(result?.as_ref());
			}
			self.parse(entity, buf, path).await
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
					.parse(entity, b"hello world".to_vec(), None)
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("hello world".into()));
	}

	/// Multiple chunks via `parse_stream` are concatenated in order.
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
					.parse_stream(entity, stream::iter(chunks), None)
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
					.parse_stream(entity, stream::iter(chunks), None)
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("🌱".into()));
	}

	/// Valid UTF-8 before a split boundary is decoded correctly after full collection.
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
					.parse_stream(entity, stream::iter(chunks), None)
					.await
					.unwrap();
				entity.get_cloned::<Value>().await.unwrap()
			})
			.await
			.xpect_eq(Value::Str("hié!".into()));
	}

	#[beet_core::test]
	async fn read_stream_truncated_utf8_at_eof() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				// Only the first byte of a 2-byte sequence, nothing follows
				let chunks: Vec<Result<Vec<u8>>> =
					vec![Ok(b"ok".to_vec()), Ok(vec![0xC3])];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks), None)
					.await
			})
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid utf-8");
	}

	/// An empty stream results in an empty string value being set.
	#[beet_core::test]
	async fn read_stream_empty() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let chunks: Vec<Result<Vec<u8>>> = vec![];
				PlainTextParser::default()
					.parse_stream(entity, stream::iter(chunks), None)
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
					.parse_stream(entity, stream::iter(chunks), None)
					.await
			})
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("stream error");
	}

	/// Parsing the same content twice does not trigger change detection.
	#[beet_core::test]
	async fn parse_same_content_no_change() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let mut parser = PlainTextParser::default();
				parser.parse(entity, b"hello".to_vec(), None).await.unwrap();
				// Retrieve the pointer to the component storage tick before re-parsing
				let v1 = entity.get_cloned::<Value>().await.unwrap();
				parser.parse(entity, b"hello".to_vec(), None).await.unwrap();
				let v2 = entity.get_cloned::<Value>().await.unwrap();
				(v1, v2)
			})
			.await
			.xpect_eq((Value::Str("hello".into()), Value::Str("hello".into())));
	}

	/// Parsing with a path attaches a [`FileSpan`] component to the entity.
	#[beet_core::test]
	async fn parse_with_path_inserts_file_span() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				PlainTextParser::default()
					.parse(
						entity,
						b"line1\nline2".to_vec(),
						Some(WsPathBuf::new("foo.txt")),
					)
					.await
					.unwrap();
				entity.get_cloned::<FileSpan>().await.unwrap()
			})
			.await
			.path()
			.xpect_eq(WsPathBuf::new("foo.txt"));
	}
}
