//! Content types for the OpenResponses API.
//!
//! This module contains types representing different content formats
//! that can appear in messages and outputs.

use super::enums::*;
use serde::Deserialize;
use serde::Serialize;

/// A text input to the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputText {
	/// The text input to the model.
	pub text: String,
}

impl InputText {
	/// Creates a new text input.
	pub fn new(text: impl Into<String>) -> Self { Self { text: text.into() } }
}

/// An image input to the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputImage {
	/// The URL of the image (fully qualified URL or base64 data URL).
	pub image_url: String,
	/// The detail level of the image.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub detail: Option<ImageDetail>,
}

impl InputImage {
	/// Creates a new image input from a URL.
	pub fn from_url(url: impl Into<String>) -> Self {
		Self {
			image_url: url.into(),
			detail: None,
		}
	}

	/// Creates a new image input from base64 data.
	pub fn from_base64(media_type: &str, data: &str) -> Self {
		Self {
			image_url: format!("data:{};base64,{}", media_type, data),
			detail: None,
		}
	}

	/// Sets the detail level.
	pub fn with_detail(mut self, detail: ImageDetail) -> Self {
		self.detail = Some(detail);
		self
	}
}

/// A file input to the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFile {
	/// The name of the file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub filename: Option<String>,
	/// The base64-encoded file data.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_data: Option<String>,
	/// The URL of the file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_url: Option<String>,
}

impl InputFile {
	/// Creates a file input from a URL.
	pub fn from_url(url: impl Into<String>) -> Self {
		Self {
			filename: None,
			file_data: None,
			file_url: Some(url.into()),
		}
	}

	/// Creates a file input from base64 data.
	pub fn from_base64(data: impl Into<String>) -> Self {
		Self {
			filename: None,
			file_data: Some(data.into()),
			file_url: None,
		}
	}

	/// Sets the filename.
	pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
		self.filename = Some(filename.into());
		self
	}
}

/// A video input to the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputVideo {
	/// A base64 or remote URL that resolves to a video file.
	pub video_url: String,
}

impl InputVideo {
	/// Creates a video input from a URL.
	pub fn from_url(url: impl Into<String>) -> Self {
		Self {
			video_url: url.into(),
		}
	}
}

/// A text output from the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputText {
	/// The text output from the model.
	pub text: String,
	/// Annotations on the text output.
	#[serde(default)]
	pub annotations: Vec<Annotation>,
	/// Log probabilities for the output tokens.
	#[serde(default)]
	pub logprobs: Vec<LogProb>,
}

impl OutputText {
	/// Creates a new output text.
	pub fn new(text: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			annotations: Vec::new(),
			logprobs: Vec::new(),
		}
	}
}

/// A refusal from the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
	/// The refusal explanation from the model.
	pub refusal: String,
}

impl Refusal {
	/// Creates a new refusal.
	pub fn new(refusal: impl Into<String>) -> Self {
		Self {
			refusal: refusal.into(),
		}
	}
}

/// An annotation that applies to a span of output text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Annotation {
	/// A citation for a web resource.
	#[serde(rename = "url_citation")]
	UrlCitation(UrlCitation),
}

/// A citation for a web resource used to generate a model response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlCitation {
	/// The URL of the web resource.
	pub url: String,
	/// The index of the first character of the citation in the message.
	pub start_index: u32,
	/// The index of the last character of the citation in the message.
	pub end_index: u32,
	/// The title of the web resource.
	pub title: String,
}

/// The log probability of a token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogProb {
	/// The token string.
	pub token: String,
	/// The log probability of this token.
	pub logprob: f64,
	/// The bytes representation of the token.
	#[serde(default)]
	pub bytes: Vec<u8>,
	/// The top log probabilities.
	#[serde(default)]
	pub top_logprobs: Vec<TopLogProb>,
}

/// A top log probability entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogProb {
	/// The token string.
	pub token: String,
	/// The log probability of this token.
	pub logprob: f64,
	/// The bytes representation of the token.
	#[serde(default)]
	pub bytes: Vec<u8>,
}

/// Reasoning text from the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningText {
	/// The reasoning text from the model.
	pub text: String,
}

/// A summary text from the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryText {
	/// A summary of the reasoning output from the model.
	pub text: String,
}

/// Content part in a message - can be input or output content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
	/// A text input.
	#[serde(rename = "input_text")]
	InputText(InputText),
	/// An image input.
	#[serde(rename = "input_image")]
	InputImage(InputImage),
	/// A file input.
	#[serde(rename = "input_file")]
	InputFile(InputFile),
	/// A video input.
	#[serde(rename = "input_video")]
	InputVideo(InputVideo),
	/// A text output.
	#[serde(rename = "output_text")]
	OutputText(OutputText),
	/// A refusal.
	Refusal(Refusal),
	/// Reasoning text.
	#[serde(rename = "reasoning_text")]
	ReasoningText(ReasoningText),
	/// Summary text.
	#[serde(rename = "summary_text")]
	SummaryText(SummaryText),
}

impl ContentPart {
	/// Creates a text input content part.
	pub fn input_text(text: impl Into<String>) -> Self {
		Self::InputText(InputText::new(text))
	}

	/// Creates an image input content part from a URL.
	pub fn input_image_url(url: impl Into<String>) -> Self {
		Self::InputImage(InputImage::from_url(url))
	}

	/// Creates an output text content part.
	pub fn output_text(text: impl Into<String>) -> Self {
		Self::OutputText(OutputText::new(text))
	}

	/// Extracts the text if this is a text content part.
	pub fn as_text(&self) -> Option<&str> {
		match self {
			Self::InputText(content) => Some(&content.text),
			Self::OutputText(content) => Some(&content.text),
			Self::ReasoningText(content) => Some(&content.text),
			Self::SummaryText(content) => Some(&content.text),
			_ => None,
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn deserializes_output_text() {
		let json = r#"{"type":"output_text","text":"Hello","annotations":[],"logprobs":[]}"#;
		let content: ContentPart = serde_json::from_str(json).unwrap();
		assert!(matches!(content, ContentPart::OutputText(_)));
		assert_eq!(content.as_text(), Some("Hello"));
	}

	#[test]
	fn deserializes_input_image() {
		let json =
			r#"{"type":"input_image","image_url":"https://example.com/img.png"}"#;
		let content: ContentPart = serde_json::from_str(json).unwrap();
		assert!(matches!(content, ContentPart::InputImage(_)));
	}

	#[test]
	fn creates_base64_image() {
		let image = InputImage::from_base64("image/png", "abc123");
		assert_eq!(image.image_url, "data:image/png;base64,abc123");
	}
}
