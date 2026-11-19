//! [`PostRef`], a post named across threads.
use crate::prelude::*;
use beet_core::prelude::*;

/// A post named across threads, `<thread id>/<post id>`: what a record holds
/// to point at another thread's post, ie [`Thread::spawned_by`] and
/// [`Thread::branch_of`], never an [`Entity`].
///
/// A string primitive, so it travels the same as a flag, a query param or a
/// body field: `--spawned-by=<thread id>/<post id>`. [`Self::parse`] is the
/// one way in.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Reflect,
	StringPrimitive,
	Serialize,
	Deserialize,
)]
#[serde(try_from = "SmolStr", into = "SmolStr")]
#[reflect(opaque)]
#[reflect(Debug, PartialEq, Hash, Serialize, Deserialize)]
pub struct PostRef(SmolStr);

impl PostRef {
	/// Name `post` of `thread`.
	pub fn new(thread: ThreadId, post: PostId) -> Self {
		Self(format!("{thread}/{post}").into())
	}

	/// Parse `<thread id>/<post id>`.
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

	/// The thread holding the post.
	pub fn thread(&self) -> ThreadId { self.split().0 }
	/// The post.
	pub fn post(&self) -> PostId { self.split().1 }

	/// Both halves, which [`Self::parse`] validated on the way in.
	fn split(&self) -> (ThreadId, PostId) {
		let (thread, post) = self.0.split_once('/').unwrap_or_default();
		(
			ThreadId::parse(thread).unwrap_or_default(),
			PostId::parse(post).unwrap_or_default(),
		)
	}

	/// Teach every authoring seam, a flag, a query param, a markup attribute,
	/// to read a [`PostRef`] and a [`ThreadId`] from their text.
	pub(crate) fn register_literals() {
		LiteralParser::register::<PostRef>(LiteralParser::new(
			|value: &Value| match value {
				Value::Str(text) => PostRef::parse(text).map(Some),
				_ => Ok(None),
			},
		));
		LiteralParser::register::<ThreadId>(LiteralParser::new(
			|value: &Value| match value {
				Value::Str(text) => ThreadId::parse(text).map(Some),
				_ => Ok(None),
			},
		));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn round_trips_its_text() {
		let thread = ThreadId::new_now();
		let post = PostId::new_now();
		let post_ref = PostRef::new(thread, post);
		PostRef::parse(post_ref.as_str())
			.unwrap()
			.xpect_eq(post_ref.clone());
		post_ref.thread().xpect_eq(thread);
		post_ref.post().xpect_eq(post);
		serde_json::to_string(&post_ref)
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
