use crate::prelude::*;
use extend::ext;
use std::path::Path;

/// Extension trait for [`Result`] providing additional conversion methods.
#[ext]
pub impl Path {
	/// Returns the media type based on the file extension, if available.
	fn media_type(&self) -> Option<MediaType> {
		let ext = self.extension()?.to_str()?;
		MediaType::from_extension(ext).xsome()
	}
}
