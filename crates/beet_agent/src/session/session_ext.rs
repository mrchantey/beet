//! Helpers for building a session, sessions are expected to
//! have the following hierarchy
//! ```
//! Session
//! 	Actor1
//! 		Message1
//! 			TextContent
//! 			FileContent
//! 		Message2
//! 			TextContent
//! 	Actor2
//! 		Message1
//! 			TextContent
//! 			..
//! ```
//!
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::path::Path;

pub async fn workspace_file(path: impl AsRef<Path>) -> Result<impl Bundle> {
	session_ext::file(AbsPathBuf::new_workspace_rel(path)?.to_string()).await
}

pub async fn file(path: impl AsRef<str>) -> Result<impl Bundle> {
	(FileContent::new(path).await?, ContentEnded::default()).xok()
}

/// Add a *completed* piece of text content, where no more
/// text will be added to this piece of content.
pub fn text(text: impl AsRef<str>) -> impl Bundle {
	(TextContent::new(text), ContentEnded::default())
}
