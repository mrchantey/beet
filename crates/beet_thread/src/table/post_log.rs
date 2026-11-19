//! Log posts: what happened to a thread rather than what was said in it.
//!
//! A [`PostIntent::Log`] post's body is a JSON object naming its kind,
//! `{"kind":"session_opened", ..}`, the rest of the object that kind's
//! fields. The intent is closed and the kinds are open: any crate adds one by
//! implementing [`LogKind`], and model input skips every log post by its
//! intent alone ([`ThreadRecords::model_posts`]), so a kind needs no
//! registration to stay out of it. A view renders a kind it knows its own way
//! and any other by its name ([`Post::log_kind`]).
use crate::prelude::*;
use beet_core::prelude::*;

/// A kind of [`PostIntent::Log`] post, its fields serialized beside its
/// [`Self::KIND`] in the post's body.
pub trait LogKind: Serialize + serde::de::DeserializeOwned {
	/// The kind's name in a log post's body. A kind outside beet_thread
	/// prefixes its crate, ie `beet_eval/decision`, so two crates never
	/// share one.
	const KIND: &'static str;
}

/// The body field naming a log post's kind.
const KIND_FIELD: &str = "kind";

impl Post {
	/// A completed log post of `log`, by `author` on `thread`.
	///
	/// ## Errors
	///
	/// Errors unless `log` serializes as a JSON object.
	pub fn new_log<T: LogKind>(
		author: ActorId,
		thread: ThreadId,
		log: &T,
	) -> Result<Self> {
		let serde_json::Value::Object(mut fields) = serde_json::to_value(log)?
		else {
			bevybail!("log kind `{}` is no JSON object", T::KIND);
		};
		fields.insert(KIND_FIELD.into(), T::KIND.into());
		Self::new_raw(
			author,
			thread,
			PostIntent::Log,
			MediaType::Json,
			serde_json::to_vec(&fields)?,
			default(),
		)
		.xok()
	}

	/// This post as a `T`, if it is a log post of that kind.
	pub fn log<T: LogKind>(&self) -> Option<T> {
		let mut fields = self.log_fields()?;
		(fields.remove(KIND_FIELD)?.as_str()? == T::KIND)
			.then(|| serde_json::from_value(fields.into()).ok())
			.flatten()
	}

	/// The kind of this post, if it is a log post.
	pub fn log_kind(&self) -> Option<SmolStr> {
		self.log_fields()?
			.get(KIND_FIELD)?
			.as_str()
			.map(SmolStr::from)
	}

	/// The body object of a log post.
	fn log_fields(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
		if self.intent() != PostIntent::Log {
			return None;
		}
		match serde_json::from_slice(self.body_bytes()).ok()? {
			serde_json::Value::Object(fields) => Some(fields),
			_ => None,
		}
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

	/// A log post reads back as its own kind and no other, and is no agent
	/// content.
	#[beet_core::test]
	fn reads_back_its_own_kind() {
		let noted = Noted { text: "hi".into() };
		let post =
			Post::new_log(ActorId::new_now(), ThreadId::new_now(), &noted)
				.unwrap();
		post.log::<Noted>().xpect_eq(Some(noted));
		post.log::<Other>().xpect_none();
		post.log_kind().xpect_eq(Some(SmolStr::from("noted")));
		post.as_agent_post().is_none().xpect_true();
	}
}
