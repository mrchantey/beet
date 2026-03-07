use beet_core::prelude::*;


/// Parses bytes into an entity's components.
///
/// Implementors provide [`NodeParser::parse`] which operates synchronously
/// on a `&mut World` and `Entity`, keeping all ECS mutations in one place.
pub trait NodeParser {
	/// Parse a complete byte buffer and apply the result to `entity`.
	///
	/// An optional `path` enables [`FileSpan`] tracking on the produced nodes.
	fn parse(
		&mut self,
		world: &mut World,
		entity: Entity,
		bytes: Vec<u8>,
		path: Option<WsPathBuf>,
	) -> Result;
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Read a plain string and collect the result via [`PlainTextParser`].
	#[beet_core::test]
	async fn read_plain_text() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let id = entity.id();
				entity
					.world()
					.with_then(move |world| {
						PlainTextParser::default()
							.parse(world, id, b"hello world".to_vec(), None)
							.unwrap();
						world.entity(id).get::<Value>().cloned().unwrap()
					})
					.await
			})
			.await
			.xpect_eq(Value::Str("hello world".into()));
	}

	/// Parsing the same content twice does not trigger change detection.
	#[beet_core::test]
	async fn parse_same_content_no_change() {
		AsyncPlugin::world()
			.run_async_local_then(|world| async move {
				let entity = world.spawn_then(()).await;
				let id = entity.id();
				entity
					.world()
					.with_then(move |world| {
						let mut parser = PlainTextParser::default();
						parser
							.parse(world, id, b"hello".to_vec(), None)
							.unwrap();
						let v1 =
							world.entity(id).get::<Value>().cloned().unwrap();
						parser
							.parse(world, id, b"hello".to_vec(), None)
							.unwrap();
						let v2 =
							world.entity(id).get::<Value>().cloned().unwrap();
						(v1, v2)
					})
					.await
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
				let id = entity.id();
				entity
					.world()
					.with_then(move |world| {
						PlainTextParser::default()
							.parse(
								world,
								id,
								b"line1\nline2".to_vec(),
								Some(WsPathBuf::new("foo.txt")),
							)
							.unwrap();
						world.entity(id).get::<FileSpan>().cloned().unwrap()
					})
					.await
			})
			.await
			.path()
			.xpect_eq(WsPathBuf::new("foo.txt"));
	}
}
