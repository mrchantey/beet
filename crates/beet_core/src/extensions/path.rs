use std::path::Path;

use crate::prelude::*;
#[cfg(target_arch = "wasm32")]
use crate::web_utils::js_runtime;
use extend::ext;

/// Extension trait for [`Result`] providing additional conversion methods.
#[ext]
pub impl Path {
	/// Returns the media type based on the file extension, if available.
	fn media_type(&self) -> Option<MediaType> {
		let ext = self.extension()?.to_str()?;
		MediaType::from_extension(ext).xsome()
	}
}
