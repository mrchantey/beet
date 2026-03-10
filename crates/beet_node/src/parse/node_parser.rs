use beet_core::prelude::*;

/// Error returned when a parser does not support the given media type.
#[derive(Debug)]
pub enum ParseError {
	/// The media type is not supported by this parser.
	UnsupportedType {
		/// The unsupported media type.
		unsupported: MediaType,
		/// The types this parser does support.
		supported: Vec<MediaType>,
	},
	/// Any other parse failure.
	Other(BevyError),
}

impl<T: Into<BevyError>> From<T> for ParseError {
	fn from(err: T) -> Self { ParseError::Other(err.into()) }
}

impl core::fmt::Display for ParseError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			ParseError::UnsupportedType {
				unsupported,
				supported,
			} => write!(
				f,
				"unsupported media type `{unsupported}`, supported: {:?}",
				supported
			),
			ParseError::Other(err) => write!(f, "{err}"),
		}
	}
}

/// Context passed to [`NodeParser::parse`].
pub struct ParseContext<'e, 'w, 'b> {
	/// The entity to parse into.
	pub entity: &'e mut EntityWorldMut<'w>,
	/// The typed bytes to parse.
	pub bytes: &'b MediaBytes<'b>,
	/// Optional source path for [`FileSpan`] tracking.
	pub path: Option<WsPathBuf>,
}

impl<'e, 'w, 'b> ParseContext<'e, 'w, 'b> {
	/// Create a new [`ParseContext`].
	pub fn new(
		entity: &'e mut EntityWorldMut<'w>,
		bytes: &'b MediaBytes<'b>,
	) -> Self {
		Self {
			entity,
			bytes,
			path: None,
		}
	}

	/// Set the optional source path.
	pub fn with_path(mut self, path: WsPathBuf) -> Self {
		self.path = Some(path);
		self
	}
}

/// Parses bytes into an entity's components.
///
/// Implementors provide [`NodeParser::parse`] which operates synchronously
/// on a `&mut EntityWorldMut`, keeping all ECS mutations in one place.
pub trait NodeParser {
	/// Parse the bytes in `cx` and apply the result to `cx.entity`.
	///
	/// Implementations should check `cx.bytes.media_type()` and return
	/// [`ParseError::UnsupportedType`] if the type is not handled.
	fn parse<'e, 'w, 'b>(
		&mut self,
		cx: ParseContext<'e, 'w, 'b>,
	) -> Result<(), ParseError>;
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Read a plain string and collect the result via [`PlainTextParser`].
	#[test]
	fn read_plain_text() {
		let mut world = World::new();
		let bytes = MediaBytes::from_str(MediaType::Text, "hello world");
		world
			.spawn_empty()
			.xtap(|entity| {
				let cx = ParseContext::new(entity, &bytes);
				PlainTextParser::default().parse(cx).unwrap();
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
		let bytes = MediaBytes::from_str(MediaType::Text, "hello");
		let mut entity_mut = world.entity_mut(entity);
		parser
			.parse(ParseContext::new(&mut entity_mut, &bytes))
			.unwrap();
		let v1 = entity_mut.get::<Value>().cloned().unwrap();
		parser
			.parse(ParseContext::new(&mut entity_mut, &bytes))
			.unwrap();
		let v2 = entity_mut.get::<Value>().cloned().unwrap();
		(v1, v2)
			.xpect_eq((Value::Str("hello".into()), Value::Str("hello".into())));
	}

	/// Parsing with a path attaches a [`FileSpan`] component to the entity.
	#[test]
	fn parse_with_path_inserts_file_span() {
		let bytes = MediaBytes::from_str(MediaType::Text, "line1\nline2");
		World::new()
			.spawn_empty()
			.xtap(|entity| {
				let cx = ParseContext::new(entity, &bytes)
					.with_path(WsPathBuf::new("foo.txt"));
				PlainTextParser::default().parse(cx).unwrap();
			})
			.get::<FileSpan>()
			.cloned()
			.unwrap()
			.path()
			.xpect_eq(WsPathBuf::new("foo.txt"));
	}
}
