//! The typed ids of thread records: [`ThreadId`] and [`PostId`], each a
//! [`Tid`] so its text sorts as its time, and [`ActorId`], an [`Rkey`].
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Declare a [`Tid`] newtype naming one kind of record.
macro_rules! tid_id {
	($(#[$meta:meta])* $name:ident) => {
		$(#[$meta])*
		#[derive(
			Debug,
			Default,
			Clone,
			Copy,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			Serialize,
			Deserialize,
			Reflect,
		)]
		#[serde(transparent)]
		#[reflect(opaque)]
		#[reflect(Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
		pub struct $name(Tid);

		impl $name {
			/// A fresh id from this process's clock.
			pub fn now() -> Self { Self(Tid::now()) }

			/// A fresh id no earlier than one microsecond past `previous`,
			/// see [`Tid::after`].
			pub fn after(previous: Self) -> Self { Self(Tid::after(previous.0)) }

			/// The id's [`Tid`].
			pub fn tid(&self) -> Tid { self.0 }

			/// The instant the id was minted at.
			pub fn created(&self) -> Timestamp { self.0.timestamp() }

			/// Parse the id's text, a [`Tid`]'s 13 characters.
			///
			/// ## Errors
			///
			/// Errors when the text is not a tid.
			pub fn parse(text: &str) -> Result<Self> {
				Tid::parse(text)
					.map_err(|err| bevyhow!("`{text}` is not an id: {err}"))?
					.xmap(Self)
					.xok()
			}

			/// Teach every authoring seam, a flag, a query param, a markup
			/// attribute, to read this id from its text.
			pub(crate) fn register_literal() {
				LiteralParser::register::<Self>(LiteralParser::new(
					|value: &Value| match value {
						Value::Str(text) => Self::parse(text).map(Some),
						_ => Ok(None),
					},
				));
			}
		}

		impl From<Tid> for $name {
			fn from(tid: Tid) -> Self { Self(tid) }
		}

		impl From<$name> for TableKey {
			fn from(id: $name) -> Self { id.0.into() }
		}

		impl std::fmt::Display for $name {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				self.0.fmt(f)
			}
		}

		impl std::str::FromStr for $name {
			type Err = BevyError;
			fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
		}
	};
}

tid_id!(
	/// A thread's id, the [`Thread`](crate::prelude::Thread) row's key and its
	/// partition's name.
	ThreadId
);

tid_id!(
	/// A post's id, the [`Post`](crate::prelude::Post) row's key, minted after
	/// every post its window knows
	/// ([`ThreadRecords`](crate::prelude::ThreadRecords)).
	PostId
);

/// An actor's id within its thread, the [`Actor`](crate::prelude::Actor)
/// row's key: an [`Rkey`], by
/// default the actor's name in kebab-case ([`Self::from_name`]).
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
)]
#[serde(transparent)]
#[reflect(opaque)]
#[reflect(Debug, PartialEq, Hash, Serialize, Deserialize)]
pub struct ActorId(Rkey);

impl ActorId {
	/// The rule an id keeps, an [`Rkey`]'s, see [`Rkey::check`].
	pub const fn check(text: &str) -> Result<(), &'static str> {
		Rkey::check(text)
	}

	/// An id from text [`check`](Self::check) passed, see [`actor_id!`].
	#[doc(hidden)]
	pub const fn from_canonical(text: &'static str) -> Self {
		Self(Rkey::from_canonical(text))
	}

	/// Parse an id, refusing anything an [`Rkey`] refuses.
	///
	/// ## Errors
	///
	/// Errors when the text is no rkey.
	pub fn parse(text: &str) -> Result<Self> {
		Rkey::parse(text)
			.map_err(|err| bevyhow!("`{text}` is not an actor id: {err}"))?
			.xmap(Self)
			.xok()
	}

	/// The id a `name` gives by default: its kebab-case, keeping only the
	/// characters an [`Rkey`] allows, ie `Coach` is `coach` and `Ann Smith`
	/// `ann-smith`.
	///
	/// ## Errors
	///
	/// Errors when nothing of the name remains, asking for an id.
	pub fn from_name(name: &str) -> Result<Self> {
		let kebab = heck::ToKebabCase::to_kebab_case(name)
			.chars()
			.filter(|char| {
				char.is_ascii_alphanumeric()
					|| matches!(char, '.' | '_' | ':' | '~' | '-')
			})
			.collect::<String>();
		let kebab = kebab.trim_matches('-');
		if kebab.is_empty() || kebab == "." || kebab == ".." {
			bevybail!(
				"the actor `{name}` has no id its name can give: give it one, ie `<CreateActor name=\"{name}\" id=\"..\"/>`"
			);
		}
		Self::parse(kebab)
	}

	/// The id as text.
	pub fn as_str(&self) -> &str { self.0.as_str() }

	/// Teach every authoring seam to read an id from its text.
	pub(crate) fn register_literal() {
		LiteralParser::register::<Self>(LiteralParser::new(|value: &Value| {
			match value {
				Value::Str(text) => Self::parse(text).map(Some),
				_ => Ok(None),
			}
		}));
	}
}

impl From<ActorId> for TableKey {
	fn from(id: ActorId) -> Self { id.0.as_str().into() }
}

impl std::fmt::Display for ActorId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl std::str::FromStr for ActorId {
	type Err = BevyError;
	fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
}

/// An [`ActorId`] known at compile time, its text a literal or a
/// `&'static str` constant checked as it compiles.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_thread::prelude::*;
/// const COACH: ActorId = actor_id!("coach");
/// COACH.as_str().xpect_eq("coach");
/// ```
///
/// ```compile_fail
/// # use beet_core::prelude::*;
/// # use beet_thread::prelude::*;
/// actor_id!("Ann Smith");
/// ```
#[macro_export]
macro_rules! actor_id {
	($text:expr) => {{
		const _: () = if let ::core::result::Result::Err(reason) =
			$crate::prelude::ActorId::check($text)
		{
			::core::panic!("{}", reason)
		};
		$crate::prelude::ActorId::from_canonical($text)
	}};
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn an_actor_id_is_its_name_in_kebab_case() {
		ActorId::from_name("Coach")
			.unwrap()
			.as_str()
			.xpect_eq("coach");
		ActorId::from_name("Ann Smith")
			.unwrap()
			.as_str()
			.xpect_eq("ann-smith");
		ActorId::from_name("日本")
			.unwrap_err()
			.to_string()
			.xpect_contains("give it one");
	}

	#[beet_core::test]
	fn ids_read_their_text() {
		let post = PostId::now();
		post.to_string().parse::<PostId>().unwrap().xpect_eq(post);
		json_ext::to_string(&post)
			.unwrap()
			.xpect_eq(format!("\"{post}\""));
		PostId::parse("nope").unwrap_err();
	}
}
