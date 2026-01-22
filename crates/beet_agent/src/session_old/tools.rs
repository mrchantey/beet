// https://platform.openai.com/docs/guides/tools
pub enum CommonTool {
	GenerateImage(GenerateImage),
}
impl Into<CommonTool> for GenerateImage {
	fn into(self) -> CommonTool { CommonTool::GenerateImage(self) }
}


// https://platform.openai.com/docs/guides/tools-image-generation#tool-options
pub struct GenerateImage {
	pub background: ImageBackground,
	pub quality: ImageQuality,
	pub size: Option<ImageSize>,
	/// Number of partial images to generate in streaming mode `0-3`
	pub partial_images: u32,
}

impl Default for GenerateImage {
	fn default() -> Self {
		Self {
			background: ImageBackground::default(),
			quality: ImageQuality::default(),
			size: None,
			partial_images: 1,
		}
	}
}

impl GenerateImage {
	pub fn with_size(self, width: u32, height: u32) -> Self {
		Self {
			size: Some(ImageSize { width, height }),
			..self
		}
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum ImageBackground {
	#[default]
	Auto,
	Opaque,
	Transparent,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum ImageQuality {
	#[default]
	Auto,
	Low,
	Medium,
	High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSize {
	/// Width in pixels
	width: u32,
	/// Height in pixels
	height: u32,
}
impl Default for ImageSize {
	fn default() -> Self {
		Self {
			width: 1024,
			height: 1024,
		}
	}
}
impl std::fmt::Display for ImageSize {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}x{}", self.width, self.height)
	}
}
