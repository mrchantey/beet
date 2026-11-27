//! Log posts: what happened to a thread rather than what was said in it.
//!
//! A log's [`PostKind`] names it and its body is that kind's fields. beet's
//! own logs are in the closed list of kinds; any other crate adds one by
//! implementing [`LogKind`], its name prefixing the crate, and it needs no
//! registration: a model reads a join ([`SessionOpened`]), an edit
//! ([`PostEdited`]) and a deletion ([`PostDeleted`]) as notes and no other
//! log of these ([`ThreadRecords::model_posts`]), and a view renders a kind
//! it knows its own way and any other by its name.
use crate::prelude::*;
use beet_core::prelude::*;

/// A kind of log post, its fields the post's body.
pub trait LogKind: Serialize + serde::de::DeserializeOwned {
	/// The kind's name, a [`PostKind`]. A kind outside beet_thread prefixes
	/// its crate, ie `beet_eval/decision`, so two crates never share one.
	const KIND: &'static str;
}

impl Post {
	/// A completed log post of `log`, by `author` on `thread`.
	///
	/// ## Errors
	///
	/// Errors unless `log` serializes as a map.
	pub fn new_log<T: LogKind>(
		author: ActorId,
		thread: ThreadId,
		log: &T,
	) -> Result<Self> {
		let Value::Map(fields) = Value::from_serde(log)? else {
			bevybail!("log kind `{}` is no map", T::KIND);
		};
		Self::new(
			author,
			thread,
			PostKind::parse(T::KIND),
			PostStatus::Completed,
			fields,
		)
		.xok()
	}

	/// This post as a `T`, if it is a log post of that kind.
	pub fn log<T: LogKind>(&self) -> Option<T> {
		(self.is_log() && self.kind().as_str() == T::KIND)
			.then(|| Value::Map(self.body().clone()).into_serde().ok())
			.flatten()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Noted {
		text: String,
	}
	impl LogKind for Noted {
		const KIND: &'static str = "noted";
	}

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Other {}
	impl LogKind for Other {
		const KIND: &'static str = "other";
	}

	/// A log post reads back as its own kind and no other, and is no
	/// [`AgentPost`].
	#[beet_core::test]
	fn reads_back_its_own_kind() {
		let noted = Noted { text: "hi".into() };
		let post =
			Post::new_log(ActorId::USER, ThreadId::now(), &noted).unwrap();
		post.log::<Noted>().xpect_eq(Some(noted));
		post.log::<Other>().xpect_none();
		post.kind().clone().xpect_eq(PostKind::parse("noted"));
		post.as_agent_post().is_none().xpect_true();
	}
}
