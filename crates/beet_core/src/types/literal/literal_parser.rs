//! [`LiteralParser`]: how human-authored input parses into a type.

use super::builtins;
use crate::prelude::*;
use alloc::sync::Arc;
use bevy::platform::sync::LazyLock;
use bevy::platform::sync::PoisonError;
use bevy::platform::sync::RwLock;
use bevy::reflect::PartialReflect;
use core::any::TypeId;

/// How human-authored input parses into one type, held in a process-wide table
/// keyed by [`TypeId`].
///
/// A `Duration` field is authored as `"30s"`, a `Timestamp` as `"2026-08-28"`,
/// a `GlobFilter` as `"guestbook.*"`. Each spelling is one entry here, consulted
/// at every seam where a human's input becomes a reflected value: a BSX
/// attribute or spread, a CLI flag, a query param, template prop verification,
/// form write-back.
///
/// The table is per PROCESS rather than per [`World`], because how a type is
/// spelled is a property of the type: two worlds disagreeing about what `"30s"`
/// means would be a bug, not a feature. That is also what keeps the seams
/// clean, since a handler parsing a request param needs no world access to do
/// it.
///
/// ## The seam contract
///
/// Every consumer follows the same three steps:
///
/// 1. build a [`Value`] from the authored input, when it maps cleanly (a
///    scalar, or a list whose items are all scalars); richer shapes skip to 3;
/// 2. parse it into the target type with [`LiteralParser::parse_type`]:
///    - `Ok(Some(reflected))` is done,
///    - `Err` is a real authoring error (a malformed glob, a unit-less
///      duration) and must NOT fall through,
///    - `Ok(None)` means no parser, or one that declined the shape, so
/// 3. fall through to the seam's structural rules (the dynamic walk, serde,
///    structural validation).
///
/// The decline arm is what lets a type with several spellings fold in with no
/// bespoke code at any seam: [`GlobFilter`]'s parser takes a string or a list of
/// strings and declines a struct literal, which then builds structurally,
/// exactly as a type with no entry would. No seam names a concrete type.
///
/// This is a parse-on-input mechanism only. It does not redefine a type's
/// schema or its serde round-trip; those stay the structural truth.
///
/// # Example
///
/// A crate teaches every seam its own type's spelling, from `Plugin::build` so
/// the entry lands before anything can coerce against it (as with any bevy
/// registration, plugin order matters):
///
/// ```
/// # use beet_core::prelude::*;
/// #[derive(Debug, Default, PartialEq, Reflect)]
/// struct Semver {
/// 	major: u32,
/// 	minor: u32,
/// }
///
/// LiteralParser::register::<Semver>(LiteralParser::new(|value: &Value| {
/// 	let Value::Str(text) = value else {
/// 		// not a string: decline, so a `{major:1,minor:2}` struct literal
/// 		// still builds structurally
/// 		return Ok(None);
/// 	};
/// 	let Some((major, minor)) = text.split_once('.') else {
/// 		bevybail!("invalid version {text:?}: expected `MAJOR.MINOR`");
/// 	};
/// 	Semver {
/// 		major: major.parse()?,
/// 		minor: minor.parse()?,
/// 	}
/// 	.xmap(Some)
/// 	.xok()
/// }));
/// ```
///
/// A foreign type registers the same way, no trait impl and so no orphan rule:
///
/// ```
/// # use beet_core::prelude::*;
/// # use core::ops::Range;
/// LiteralParser::register::<Range<u32>>(LiteralParser::new(|value: &Value| {
/// 	let Value::Str(text) = value else {
/// 		return Ok(None);
/// 	};
/// 	let Some((start, end)) = text.split_once("..") else {
/// 		bevybail!("invalid range {text:?}: expected `START..END`");
/// 	};
/// 	let (start, end): (u32, u32) = (start.parse()?, end.parse()?);
/// 	(start..end).xmap(Some).xok()
/// }));
/// ```
#[derive(Clone)]
pub struct LiteralParser {
	/// `Ok(Some)` parsed, `Ok(None)` declines the shape (the structural path
	/// takes over), `Err` is a real authoring error and must not fall through.
	parse: Arc<
		dyn Fn(&Value) -> Result<Option<Box<dyn PartialReflect>>> + Send + Sync,
	>,
	/// How a human writes this type, for a surface that must describe a value
	/// it cannot show (see [`LiteralParser::hint`]).
	hint: Option<&'static str>,
}

impl core::fmt::Debug for LiteralParser {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str("LiteralParser")
	}
}

impl LiteralParser {
	/// Build a parser from a typed one, erasing its output for the table.
	pub fn new<T: 'static + Send + Sync + PartialReflect>(
		parse: impl 'static + Send + Sync + Fn(&Value) -> Result<Option<T>>,
	) -> Self {
		Self {
			parse: Arc::new(move |value| {
				parse(value).map(|parsed| {
					parsed.map(|parsed| {
						Box::new(parsed) as Box<dyn PartialReflect>
					})
				})
			}),
			hint: None,
		}
	}

	/// Describe how this type is written, for a surface that names a value it
	/// cannot show: `--help` renders it as the param's `kind`, so a
	/// `--created` flag reads as `a YYYY-MM-DD date` rather than as
	/// `beet_core::utils::timestamp::Timestamp`.
	///
	/// Only worth setting where the type's own name does not already say it: a
	/// `u16` or a `String` needs none.
	pub fn with_hint(mut self, hint: &'static str) -> Self {
		self.hint = Some(hint);
		self
	}

	/// How a human writes this type, when [`with_hint`](Self::with_hint) said.
	pub fn hint(&self) -> Option<&'static str> { self.hint }

	/// Teach every authoring seam how `T` is spelled, replacing any parser `T`
	/// already had (including a builtin, so an app may override one).
	pub fn register<T: 'static>(parser: Self) {
		Self::table()
			.write()
			.unwrap_or_else(PoisonError::into_inner)
			.insert(TypeId::of::<T>(), parser);
	}

	/// The parser registered for `type_id`.
	///
	/// Cloned out rather than borrowed, so the table's lock is never held
	/// across a parse: an entry is one [`Arc`], and a parser is free to call
	/// back into the table for a type it wraps.
	pub fn get(type_id: TypeId) -> Option<Self> {
		Self::table()
			.read()
			.unwrap_or_else(PoisonError::into_inner)
			.get(&type_id)
			.cloned()
	}

	/// Parse `value` into `type_id`, the one call every seam makes.
	///
	/// `Ok(None)` covers both "no parser" and "the parser declined this shape",
	/// which the seam contract treats identically: fall through to the
	/// structural rules.
	pub fn parse_type(
		type_id: TypeId,
		value: &Value,
	) -> Result<Option<Box<dyn PartialReflect>>> {
		match Self::get(type_id) {
			Some(parser) => parser.parse(value),
			None => Ok(None),
		}
	}

	/// Parse `value` through this parser, per the seam contract on
	/// [`LiteralParser`].
	pub fn parse(
		&self,
		value: &Value,
	) -> Result<Option<Box<dyn PartialReflect>>> {
		(self.parse)(value)
	}

	/// The process-wide table, seeded with the builtins on first touch.
	fn table() -> &'static RwLock<HashMap<TypeId, Self>> {
		static TABLE: LazyLock<RwLock<HashMap<TypeId, LiteralParser>>> =
			LazyLock::new(|| RwLock::new(builtins::builtins()));
		&TABLE
	}
}
