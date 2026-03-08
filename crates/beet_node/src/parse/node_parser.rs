use beet_core::prelude::*;


/// Parses bytes into an entity's components.
///
/// Implementors provide [`NodeParser::parse`] which operates synchronously
/// on a `&mut EntityWorldMut`, keeping all ECS mutations in one place.
pub trait NodeParser {
	/// Parse a complete byte buffer and apply the result to `entity`.
	///
	/// Accepts anything that can be viewed as a byte slice, ie `&str`,
	/// `&[u8]`, `Vec<u8>`, `&String`, etc.
	///
	/// An optional `path` enables [`FileSpan`] tracking on the produced nodes.
	fn parse(
		&mut self,
		entity: &mut EntityWorldMut,
		bytes: &[u8],
		path: Option<WsPathBuf>,
	) -> Result;
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Read a plain string and collect the result via [`PlainTextParser`].
	#[test]
	fn read_plain_text() {
		let mut world = World::new();
		world
			.spawn_empty()
			.xtap(|entity| {
				PlainTextParser::default()
					.parse(entity, b"hello world", None)
					.unwrap();
			})
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello world".into()));
	}

	/// Parsing the same content twice does not trigger change detection.
	#[test]
	fn parse_same_content_no_change() {
		let mut world = World::new();
		let mut parser = PlainTextParser::default();
		let entity = world.spawn_empty().id();
		let mut entity_mut = world.entity_mut(entity);
		parser.parse(&mut entity_mut, b"hello", None).unwrap();
		let v1 = entity_mut.get::<Value>().cloned().unwrap();
		parser.parse(&mut entity_mut, b"hello", None).unwrap();
		let v2 = entity_mut.get::<Value>().cloned().unwrap();
		(v1, v2)
			.xpect_eq((Value::Str("hello".into()), Value::Str("hello".into())));
	}

	/// Parsing with a path attaches a [`FileSpan`] component to the entity.
	#[test]
	fn parse_with_path_inserts_file_span() {
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				PlainTextParser::default()
					.parse(
						entity,
						b"line1\nline2",
						Some(WsPathBuf::new("foo.txt")),
					)
					.unwrap();
			})
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(WsPathBuf::new("foo.txt"));
	}
}
