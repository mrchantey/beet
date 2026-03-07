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
	#[test]
	fn read_plain_text() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		PlainTextParser::default()
			.parse(&mut world, entity, b"hello world".to_vec(), None)
			.unwrap();
		world
			.entity(entity)
			.get::<Value>()
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello world".into()));
	}

	/// Parsing the same content twice does not trigger change detection.
	#[test]
	fn parse_same_content_no_change() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		let mut parser = PlainTextParser::default();
		parser
			.parse(&mut world, entity, b"hello".to_vec(), None)
			.unwrap();
		let v1 = world.entity(entity).get::<Value>().cloned().unwrap();
		parser
			.parse(&mut world, entity, b"hello".to_vec(), None)
			.unwrap();
		let v2 = world.entity(entity).get::<Value>().cloned().unwrap();
		(v1, v2)
			.xpect_eq((Value::Str("hello".into()), Value::Str("hello".into())));
	}

	/// Parsing with a path attaches a [`FileSpan`] component to the entity.
	#[test]
	fn parse_with_path_inserts_file_span() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		PlainTextParser::default()
			.parse(
				&mut world,
				entity,
				b"line1\nline2".to_vec(),
				Some(WsPathBuf::new("foo.txt")),
			)
			.unwrap();
		world
			.entity(entity)
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(WsPathBuf::new("foo.txt"));
	}
}
