//! [`FenceInfo`]: what a fence says beyond the format it holds.
use beet_core::prelude::*;

/// The rest of a fence's info string, after the media type naming the format
/// it holds, ie `text` of ```` ```mermaid text ````. A
/// [setting](DocParsers::setting) of the registry a fence's document parses
/// against, empty anywhere else.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deref)]
pub struct FenceInfo(SmolStr);

impl FenceInfo {
	/// The info string after its media type.
	pub fn new(info: impl Into<SmolStr>) -> Self { Self(info.into()) }

	/// The info a document parses with against `parsers`, empty for one that
	/// is no fence's.
	pub fn of(parsers: &DocParsers) -> Self {
		parsers.setting::<Self>().cloned().unwrap_or_default()
	}
}
