use crate::prelude::*;
use beet_core::prelude::*;

/// Renders an entity tree into a [`RenderOutput`].
///
/// Implementors walk the entity tree rooted at `entity` using the
/// provided [`NodeWalker`] and produce either serialized media bytes,
/// a state change notification, or an unchanged signal.
pub trait NodeRenderer {
	/// Render the entity tree rooted at `entity`.
	fn render(&mut self, walker: &NodeWalker, entity: Entity) -> RenderOutput;
}

/// The result of a [`NodeRenderer::render`] call.
pub enum RenderOutput {
	/// The output was rendered to bytes,
	/// ie html, markdown, json.
	Media {
		media_type: MediaType,
		bytes: Vec<u8>,
	},
	/// The render step involved changes
	/// to the state of this renderer,
	/// indicating it should be redrawn etc.
	StateChange,
	/// The render step involved no changes,
	/// no action is required.
	StateUnchanged,
}

impl RenderOutput {
	/// Convenience constructor for a [`RenderOutput::Media`] with the given
	/// media type and UTF-8 string content.
	pub fn media_string(media_type: MediaType, content: String) -> Self {
		Self::Media {
			media_type,
			bytes: content.into_bytes(),
		}
	}

	/// Try to interpret the media bytes as a UTF-8 string.
	///
	/// Returns `None` for non-media variants or invalid UTF-8.
	pub fn as_str(&self) -> Option<&str> {
		match self {
			Self::Media { bytes, .. } => std::str::from_utf8(bytes).ok(),
			_ => None,
		}
	}

	/// Returns `true` if this is a [`RenderOutput::Media`] variant.
	pub fn is_media(&self) -> bool { matches!(self, Self::Media { .. }) }

	/// Returns `true` if this is a [`RenderOutput::StateChange`] variant.
	pub fn is_state_change(&self) -> bool { matches!(self, Self::StateChange) }

	/// Returns `true` if this is a [`RenderOutput::StateUnchanged`] variant.
	pub fn is_state_unchanged(&self) -> bool {
		matches!(self, Self::StateUnchanged)
	}
}

impl core::fmt::Display for RenderOutput {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		match self {
			Self::Media { bytes, .. } => match std::str::from_utf8(bytes) {
				Ok(text) => write!(formatter, "{text}"),
				Err(_) => write!(formatter, "<{} bytes>", bytes.len()),
			},
			Self::StateChange => write!(formatter, "Changed"),
			Self::StateUnchanged => write!(formatter, "Unchanged"),
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn display_media_utf8() {
		RenderOutput::media_string(MediaType::Html, "hello".into())
			.to_string()
			.xpect_eq("hello".to_string());
	}

	#[test]
	fn display_media_binary() {
		RenderOutput::Media {
			media_type: MediaType::Bytes,
			bytes: vec![0xFF, 0xFE],
		}
		.to_string()
		.xpect_eq("<2 bytes>".to_string());
	}

	#[test]
	fn display_state_change() {
		RenderOutput::StateChange
			.to_string()
			.xpect_eq("Changed".to_string());
	}

	#[test]
	fn display_state_unchanged() {
		RenderOutput::StateUnchanged
			.to_string()
			.xpect_eq("Unchanged".to_string());
	}

	#[test]
	fn as_str_media() {
		RenderOutput::media_string(MediaType::Text, "hello".into())
			.as_str()
			.unwrap()
			.xpect_eq("hello");
	}

	#[test]
	fn as_str_non_media() {
		RenderOutput::StateChange.as_str().is_none().xpect_true();
	}
}
