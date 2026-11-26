//! [`PostRef`], a post named across threads.
use crate::prelude::*;
use beet_core::prelude::*;

/// A post named across threads: what a record holds to point at another
/// thread's post, ie [`Thread::spawned_by`] and [`Thread::branch_of`], never
/// an [`Entity`].
///
/// Its text form, `<thread id>/<post id>`, is how it travels as a flag, a
/// query param or a body field (`--spawned-by=<thread id>/<post id>`), and
/// its serde form is that same string, as with [`Url`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Get)]
#[reflect(opaque)]
#[reflect(Debug, PartialEq, Hash, Serialize, Deserialize)]
#[get(copy)]
pub struct PostRef {
	/// The thread holding the post.
	thread: ThreadId,
	/// The post.
	post: PostId,
}

impl PostRef {
	/// Name `post` of `thread`.
	pub fn new(thread: ThreadId, post: PostId) -> Self { Self { thread, post } }

	/// Parse the text form, `<thread id>/<post id>`.
	///
	/// ## Errors
	///
	/// Errors unless the text is two uuids joined by one `/`.
	pub fn parse(text: &str) -> Result<Self> {
		let (thread, post) = text.split_once('/').ok_or_else(|| {
			bevyhow!("`{text}` names no post: expected `<thread id>/<post id>`")
		})?;
		Self::new(ThreadId::parse(thread)?, PostId::parse(post)?).xok()
	}

	/// Teach every authoring seam, a flag, a query param, a markup attribute,
	/// to read a [`PostRef`] from its text.
	pub(crate) fn register_literal() {
		LiteralParser::register::<PostRef>(LiteralParser::new(
			|value: &Value| match value {
				Value::Str(text) => PostRef::parse(text).map(Some),
				_ => Ok(None),
			},
		));
	}
}

impl std::fmt::Display for PostRef {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}/{}", self.thread, self.post)
	}
}

impl Serialize for PostRef {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		serializer.collect_str(self)
	}
}

impl<'de> Deserialize<'de> for PostRef {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		let text = SmolStr::deserialize(deserializer)?;
		Self::parse(&text).map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn round_trips_its_text() {
		let thread = ThreadId::now();
		let post = PostId::now();
		let post_ref = PostRef::new(thread, post);
		PostRef::parse(&post_ref.to_string())
			.unwrap()
			.xpect_eq(post_ref);
		json_ext::to_string(&post_ref)
			.unwrap()
			.xpect_eq(format!("\"{thread}/{post}\""));
	}

	#[beet_core::test]
	fn refuses_anything_else() {
		PostRef::parse("loop")
			.unwrap_err()
			.to_string()
			.xpect_contains("names no post");
		PostRef::parse("a/b").unwrap_err();
	}
}
