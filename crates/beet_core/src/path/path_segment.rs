//! [`PathSegment`]: one plain segment of a [`FwdPath`].
use crate::prelude::*;

/// One plain segment of a path: never empty, holding no `/` or `\`, and
/// neither `.` nor `..`. Joined onto a [`FwdPath`] it names one entry
/// directly below it, never climbing out ([`FwdPath::join_segment`]).
///
/// Text parses through [`parse`](Self::parse); a name built from values that
/// always render as one segment is written with [`path_segment!`], whose
/// template is checked as it compiles, so it needs no `Result`:
///
/// ```
/// # use beet_core::prelude::*;
/// let size = 512u32;
/// path_segment!("icon-{}.png", size).as_str().xpect_eq("icon-512.png");
/// PathSegment::parse("icons/a.png").unwrap_err();
/// PathSegment::parse("..").unwrap_err();
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct PathSegment(SmolStr);

impl PathSegment {
	/// Parse one plain segment.
	///
	/// # Errors
	/// Errors on the empty text, a `/` or `\`, or `.` or `..`.
	pub fn parse(text: &str) -> Result<Self> {
		match Self::is_canonical(text) {
			true => Self(SmolStr::new(text)).xok(),
			false => bevybail!(
				"`{text}` is no path segment: a segment is never empty, holds \
				 no `/` or `\\`, and is neither `.` nor `..`"
			),
		}
	}

	/// The segment `value` renders as, for a type that always renders as
	/// one, see [`SegmentText`].
	pub fn of(value: &impl SegmentText) -> Self {
		Self(SmolStr::new(value.to_string()))
	}

	/// Whether `text` is one plain segment, the check [`path_segment!`] runs
	/// on a literal as it compiles.
	pub const fn is_canonical(text: &str) -> bool {
		let bytes = text.as_bytes();
		match bytes {
			[] | [b'.'] | [b'.', b'.'] => return false,
			_ => {}
		}
		let mut index = 0;
		while index < bytes.len() {
			if bytes[index] == b'/' || bytes[index] == b'\\' {
				return false;
			}
			index += 1;
		}
		true
	}

	/// Whether `template` is a [`path_segment!`] template: no `/` or `\`, and
	/// every placeholder positional, `{}` or an integer's `{:016x}`, so each
	/// value is one the macro checks is a [`SegmentText`]. A template with no
	/// placeholder is a whole segment, see [`is_canonical`](Self::is_canonical).
	pub const fn is_template(template: &str) -> bool {
		let bytes = template.as_bytes();
		let mut placeholders = 0;
		let mut index = 0;
		while index < bytes.len() {
			match bytes[index] {
				b'/' | b'\\' => return false,
				b'{' if index + 1 < bytes.len() && bytes[index + 1] == b'{' => {
					index += 1;
				}
				b'}' if index + 1 < bytes.len() && bytes[index + 1] == b'}' => {
					index += 1;
				}
				b'{' => {
					index += 1;
					// a spec only of width and radix flags, never a name, a
					// position, a fill or a precision that could empty a value
					if index < bytes.len() && bytes[index] == b':' {
						index += 1;
						while index < bytes.len()
							&& matches!(
								bytes[index],
								b'0'..=b'9' | b'#' | b'x' | b'X' | b'b' | b'o'
							) {
							index += 1;
						}
					}
					if index >= bytes.len() || bytes[index] != b'}' {
						return false;
					}
					placeholders += 1;
				}
				_ => {}
			}
			index += 1;
		}
		placeholders > 0 || Self::is_canonical(template)
	}

	/// Wrap text already one plain segment, ie a segment of a [`FwdPath`].
	pub(crate) fn new_unchecked(text: impl Into<SmolStr>) -> Self {
		Self(text.into())
	}

	/// A segment from a literal [`is_canonical`](Self::is_canonical) has
	/// checked, see [`path_segment!`].
	#[doc(hidden)]
	pub const fn from_canonical(text: &'static str) -> Self {
		Self(SmolStr::new_static(text))
	}

	/// A segment formatted from an [`is_template`](Self::is_template) template
	/// of [`SegmentText`] values, see [`path_segment!`].
	#[doc(hidden)]
	pub fn from_template(text: String) -> Self { Self(SmolStr::new(text)) }

	/// `value` unchanged, requiring it be a [`SegmentText`], so
	/// [`path_segment!`] refuses any other value as it compiles.
	#[doc(hidden)]
	pub fn checked<T: SegmentText>(value: &T) -> &T { value }
}

/// A type whose text is always one plain [`PathSegment`]: never empty,
/// holding no `/` or `\`, and neither `.` nor `..`. What [`path_segment!`]
/// accepts as a template's values.
///
/// Implement it only for a type whose every value renders so, ie a number, a
/// uuid, a date or an atproto identifier, never one with an empty or
/// free-form value.
pub trait SegmentText: core::fmt::Display {}

impl SegmentText for PathSegment {}
impl SegmentText for Date {}
impl SegmentText for Tid {}
impl SegmentText for Rkey {}
impl SegmentText for Nsid {}
impl SegmentText for Did {}
#[cfg(feature = "uuid")]
impl SegmentText for uuid::Uuid {}
impl SegmentText for u8 {}
impl SegmentText for u16 {}
impl SegmentText for u32 {}
impl SegmentText for u64 {}
impl SegmentText for u128 {}
impl SegmentText for usize {}
impl SegmentText for i8 {}
impl SegmentText for i16 {}
impl SegmentText for i32 {}
impl SegmentText for i64 {}
impl SegmentText for i128 {}
impl SegmentText for isize {}

impl From<PathSegment> for FwdPath {
	fn from(segment: PathSegment) -> Self {
		FwdPath::new_unchecked(SmolPath::new_unchecked(segment.0))
	}
}

/// A [`PathSegment`] known at compile time: a literal, or a template of
/// [`SegmentText`] values, checked as it compiles so it needs no `Result`.
/// A template takes positional `{}` placeholders, and an integer's
/// `{:016x}`, never a name captured from scope.
///
/// ```
/// # use beet_core::prelude::*;
/// const INDEX: PathSegment = path_segment!("index.png");
/// let date = Date::from_civil(2026, 11, 28).unwrap();
/// path_segment!("{}.db", date).as_str().xpect_eq("2026-11-28.db");
/// ```
///
/// ```compile_fail
/// # use beet_core::prelude::*;
/// path_segment!("icons/{}.png", 512u32);
/// ```
///
/// ```compile_fail
/// # use beet_core::prelude::*;
/// let name = "../escape";
/// path_segment!("{}.png", name);
/// ```
#[macro_export]
macro_rules! path_segment {
	($text:literal) => {{
		const _: () = ::core::assert!(
			$crate::prelude::PathSegment::is_canonical($text),
			"a `path_segment!` literal is one plain segment: never empty, no `/` or `\\`, and neither `.` nor `..`"
		);
		$crate::prelude::PathSegment::from_canonical($text)
	}};
	($template:literal, $($value:expr),+ $(,)?) => {{
		const _: () = ::core::assert!(
			$crate::prelude::PathSegment::is_template($template),
			"a `path_segment!` template holds no `/` or `\\`, and only positional `{{}}` placeholders"
		);
		$crate::prelude::PathSegment::from_template($crate::_alloc::format!(
			$template,
			$($crate::prelude::PathSegment::checked(&$value)),+
		))
	}};
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn parses_one_segment() {
		PathSegment::parse("icon.png").unwrap().as_str().xpect_eq("icon.png");
		PathSegment::parse("...").unwrap();
		for invalid in ["", ".", "..", "a/b", "a\\b"] {
			PathSegment::parse(invalid).unwrap_err();
		}
	}

	#[crate::test]
	fn checks_a_template() {
		for valid in ["{}", "icon-{}.png", "{:016x}.bin", "{}{}", "{{}}x", "a"] {
			PathSegment::is_template(valid).xpect_true();
		}
		for invalid in ["a/{}", "{name}", "{0}", "{:.0}", "{:/>4}", "", ".", "{"] {
			PathSegment::is_template(invalid).xpect_false();
		}
	}

	#[crate::test]
	fn formats_a_template() {
		path_segment!("{:016x}.bin", 255u64)
			.as_str()
			.xpect_eq("00000000000000ff.bin");
		fwd_path!("icons")
			.join_segment(&path_segment!("icon-{}.png", 192u32))
			.xpect_eq(fwd_path!("icons/icon-192.png"));
	}
}
