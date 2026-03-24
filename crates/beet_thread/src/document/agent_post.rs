use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// ═══════════════════════════════════════════════════════════════════════
// PostStatus
// ═══════════════════════════════════════════════════════════════════════

/// Whether a post is complete, in-progress, or interrupted.
///
/// Encoded in post metadata. Default (no fields) = [`Completed`](PostStatus::Completed).
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum PostStatus {
	Completed,
	Interrupted,
	InProgress,
}

/// Reads [`PostStatus`] from post metadata.
pub fn post_status(post: &Post) -> PostStatus {
	let interrupted = post
		.metadata()
		.get("interrupted")
		.and_then(|val| val.as_bool())
		.unwrap_or(false);
	let in_progress = post
		.metadata()
		.get("in_progress")
		.and_then(|val| val.as_bool())
		.unwrap_or(false);
	if interrupted {
		PostStatus::Interrupted
	} else if in_progress {
		PostStatus::InProgress
	} else {
		PostStatus::Completed
	}
}

/// Writes [`PostStatus`] into post metadata.
pub fn set_post_status(post: &mut Post, status: PostStatus) {
	let map = post.metadata_mut();
	match status {
		PostStatus::Completed => {
			map.remove("interrupted");
			map.remove("in_progress");
		}
		PostStatus::Interrupted => {
			map.insert("interrupted".into(), true.into());
			map.remove("in_progress");
		}
		PostStatus::InProgress => {
			map.insert("in_progress".into(), true.into());
			map.remove("interrupted");
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// AgentPost
// ═══════════════════════════════════════════════════════════════════════

/// A typed projection of a [`Post`] into one of the known agent content
/// variants. Checks for validity are performed during construction.
pub enum AgentPost<'a> {
	Text(TextView<'a>),
	Refusal(RefusalView<'a>),
	Url(UrlView<'a>),
	Bytes(BytesView<'a>),
	Error(ErrorView<'a>),
	FunctionCall(FunctionCallView<'a>),
	FunctionCallOutput(FunctionCallOutputView<'a>),
}

impl<'a> AgentPost<'a> {
	/// Classifies the post into the most specific view variant.
	/// Falls back to [`Text`](AgentPost::Text) for text-like posts
	/// and [`Bytes`](AgentPost::Bytes) for everything else.
	pub fn new(post: &'a Post) -> Self {
		if let Some(fc) = FunctionCallView::try_new(post) {
			return AgentPost::FunctionCall(fc);
		}
		if let Some(fco) = FunctionCallOutputView::try_new(post) {
			return AgentPost::FunctionCallOutput(fco);
		}
		if post.intent().is_server_error() {
			return AgentPost::Error(ErrorView { post });
		}
		if let Some(view) = RefusalView::try_new(post) {
			return AgentPost::Refusal(view);
		}
		if let Some(view) = UrlView::try_new(post) {
			return AgentPost::Url(view);
		}
		if post.media_type().is_text() {
			return AgentPost::Text(TextView { post });
		}
		AgentPost::Bytes(BytesView { post })
	}

	/// Returns the underlying post reference.
	pub fn post(&self) -> &'a Post {
		match self {
			AgentPost::Text(view) => view.post(),
			AgentPost::Refusal(view) => view.post(),
			AgentPost::Url(view) => view.post(),
			AgentPost::Bytes(view) => view.post(),
			AgentPost::Error(view) => view.post(),
			AgentPost::FunctionCall(view) => view.post(),
			AgentPost::FunctionCallOutput(view) => view.post(),
		}
	}

	/// Whether this post should usually be surfaced to the user.
	pub fn is_display(&self) -> bool { self.post().intent().is_display() }
}


// ═══════════════════════════════════════════════════════════════════════
// View types
// ═══════════════════════════════════════════════════════════════════════

/// A text content post (OK intent, text-like media type).
pub struct TextView<'a> {
	post: &'a Post,
}

impl<'a> TextView<'a> {
	pub fn try_new(post: &'a Post) -> Option<Self> {
		if post.media_type().is_text() && post.intent() == PostIntent::OK {
			Some(Self { post })
		} else {
			None
		}
	}
	pub fn text(&self) -> &str {
		self.post
			.as_str()
			.expect("text view validated on construction")
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl TextView<'static> {
	/// Creates a new text post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		text: impl Into<String>,
	) -> Post {
		Post::new_raw(
			author,
			thread,
			PostIntent::OK,
			MediaType::Text,
			text.into().into_bytes(),
			serde_json::Map::new(),
		)
	}
}

/// A refusal post (REFUSAL intent).
pub struct RefusalView<'a> {
	post: &'a Post,
}

impl<'a> RefusalView<'a> {
	pub fn try_new(post: &'a Post) -> Option<Self> {
		(post.intent() == PostIntent::REFUSAL).then_some(Self { post })
	}
	pub fn text(&self) -> &str {
		self.post
			.as_str()
			.expect("refusal view validated on construction")
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl RefusalView<'static> {
	/// Creates a new refusal post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		text: impl Into<String>,
	) -> Post {
		Post::new_raw(
			author,
			thread,
			PostIntent::REFUSAL,
			MediaType::Text,
			text.into().into_bytes(),
			serde_json::Map::new(),
		)
	}
}

/// A URL post (URL media type).
pub struct UrlView<'a> {
	post: &'a Post,
}

impl<'a> UrlView<'a> {
	pub fn try_new(post: &'a Post) -> Option<Self> {
		(post.media_type() == &MediaType::Url).then_some(Self { post })
	}
	pub fn url(&self) -> &str {
		self.post
			.as_str()
			.expect("url view validated on construction")
	}
	pub fn file_stem(&self) -> Option<&str> {
		self.post
			.metadata()
			.get("file_stem")
			.and_then(|val| val.as_str())
	}
	/// Constructs a filename from metadata `file_stem` and the media type
	/// extension.
	pub fn filename(&self) -> Option<String> {
		let file_stem = self.file_stem().unwrap_or("file");
		if let Some(ext) = self.post.media_type().extension() {
			Some(format!("{file_stem}.{ext}"))
		} else {
			Some(file_stem.to_string())
		}
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl UrlView<'static> {
	/// Creates a new URL post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		url: impl Into<String>,
		file_stem: Option<String>,
	) -> Post {
		let mut metadata = serde_json::Map::new();
		if let Some(stem) = file_stem {
			metadata
				.insert("file_stem".into(), serde_json::Value::String(stem));
		}
		Post::new_raw(
			author,
			thread,
			PostIntent::OK,
			MediaType::Url,
			url.into().into_bytes(),
			metadata,
		)
	}
}

/// A binary/media post.
pub struct BytesView<'a> {
	post: &'a Post,
}

impl<'a> BytesView<'a> {
	pub fn try_new(post: &'a Post) -> Option<Self> {
		(!post.media_type().is_text() && post.media_type() != &MediaType::Url)
			.then_some(Self { post })
	}
	pub fn bytes(&self) -> &[u8] { self.post.body_bytes() }
	pub fn file_stem(&self) -> Option<&str> {
		self.post
			.metadata()
			.get("file_stem")
			.and_then(|val| val.as_str())
	}
	/// Constructs a filename from metadata `file_stem` and the media type
	/// extension.
	pub fn filename(&self) -> Option<String> {
		let file_stem = self.file_stem().unwrap_or("file");
		if let Some(ext) = self.post.media_type().extension() {
			Some(format!("{file_stem}.{ext}"))
		} else {
			Some(file_stem.to_string())
		}
	}
	/// Returns the body as base64.
	pub fn body_base64(&self) -> String {
		base64::Engine::encode(
			&base64::prelude::BASE64_STANDARD,
			self.post.body_bytes(),
		)
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl BytesView<'static> {
	/// Creates a new binary/media post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		media_type: MediaType,
		bytes: Vec<u8>,
		file_stem: Option<String>,
	) -> Post {
		let mut metadata = serde_json::Map::new();
		if let Some(stem) = file_stem {
			metadata
				.insert("file_stem".into(), serde_json::Value::String(stem));
		}
		Post::new_raw(
			author,
			thread,
			PostIntent::OK,
			media_type,
			bytes,
			metadata,
		)
	}
}

/// An error post (5xx intent).
pub struct ErrorView<'a> {
	post: &'a Post,
}

impl<'a> ErrorView<'a> {
	pub fn try_new(post: &'a Post) -> Option<Self> {
		post.intent().is_server_error().then_some(Self { post })
	}
	pub fn message(&self) -> &str {
		self.post.as_str().unwrap_or("[non-utf8 error]")
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl ErrorView<'static> {
	/// Creates a new error post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		message: impl Into<String>,
	) -> Post {
		Post::new_raw(
			author,
			thread,
			PostIntent::INTERNAL_ERROR,
			MediaType::Text,
			message.into().into_bytes(),
			serde_json::Map::new(),
		)
	}
}

/// A function call post. Validated by `post_kind: "function_call"` metadata.
pub struct FunctionCallView<'a> {
	post: &'a Post,
}

impl<'a> FunctionCallView<'a> {
	/// Attempts to create a view, returning `None` if the post
	/// is not a function call.
	pub fn try_new(post: &'a Post) -> Option<Self> {
		post.metadata()
			.get("post_kind")
			.and_then(|val| val.as_str())
			.filter(|kind| *kind == "function_call")
			.map(|_| Self { post })
	}
	/// The function name.
	pub fn name(&self) -> &str {
		self.post
			.metadata()
			.get("fc_name")
			.and_then(|val| val.as_str())
			.expect("checked on construction")
	}
	/// The unique call identifier.
	pub fn call_id(&self) -> &str {
		self.post
			.metadata()
			.get("fc_id")
			.and_then(|val| val.as_str())
			.expect("checked on construction")
	}
	/// The arguments as a JSON string (the body).
	pub fn arguments(&self) -> &str {
		self.post
			.as_str()
			.expect("function call body should be valid utf-8")
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl FunctionCallView<'static> {
	/// Creates a new function call post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		name: impl Into<String>,
		call_id: impl Into<String>,
		arguments: impl Into<String>,
	) -> Post {
		let mut metadata = serde_json::Map::new();
		metadata.insert("post_kind".into(), "function_call".into());
		metadata
			.insert("fc_name".into(), serde_json::Value::String(name.into()));
		metadata
			.insert("fc_id".into(), serde_json::Value::String(call_id.into()));
		Post::new_raw(
			author,
			thread,
			PostIntent::OK,
			MediaType::Json,
			arguments.into().into_bytes(),
			metadata,
		)
	}
}

/// A function call output post. Validated by `post_kind: "function_call_output"` metadata.
pub struct FunctionCallOutputView<'a> {
	post: &'a Post,
}

impl<'a> FunctionCallOutputView<'a> {
	/// Attempts to create a view, returning `None` if the post
	/// is not a function call output.
	pub fn try_new(post: &'a Post) -> Option<Self> {
		post.metadata()
			.get("post_kind")
			.and_then(|val| val.as_str())
			.filter(|kind| *kind == "function_call_output")
			.map(|_| Self { post })
	}
	/// The unique call identifier.
	pub fn call_id(&self) -> &str {
		self.post
			.metadata()
			.get("fc_id")
			.and_then(|val| val.as_str())
			.expect("checked on construction")
	}
	/// The function name, if available.
	pub fn name(&self) -> Option<&str> {
		self.post
			.metadata()
			.get("fc_name")
			.and_then(|val| val.as_str())
	}
	/// The output string (the body).
	pub fn output(&self) -> &str {
		self.post
			.as_str()
			.expect("function call output body should be valid utf-8")
	}
	pub fn post(&self) -> &'a Post { self.post }
}

impl FunctionCallOutputView<'static> {
	/// Creates a new function call output post.
	pub fn into_post(
		author: ActorId,
		thread: ThreadId,
		call_id: impl Into<String>,
		output: impl Into<String>,
		fc_name: Option<String>,
	) -> Post {
		let mut metadata = serde_json::Map::new();
		metadata.insert("post_kind".into(), "function_call_output".into());
		metadata
			.insert("fc_id".into(), serde_json::Value::String(call_id.into()));
		if let Some(name) = fc_name {
			metadata.insert("fc_name".into(), serde_json::Value::String(name));
		}
		Post::new_raw(
			author,
			thread,
			PostIntent::OK,
			MediaType::Json,
			output.into().into_bytes(),
			metadata,
		)
	}
}


// ═══════════════════════════════════════════════════════════════════════
// Reasoning post constructors
// ═══════════════════════════════════════════════════════════════════════

/// Creates a reasoning content post.
pub fn reasoning_content_post(
	author: ActorId,
	thread: ThreadId,
	text: impl Into<String>,
) -> Post {
	Post::new_raw(
		author,
		thread,
		PostIntent::REASONING_CONTENT,
		MediaType::Text,
		text.into().into_bytes(),
		serde_json::Map::new(),
	)
}

/// Creates a reasoning summary post.
pub fn reasoning_summary_post(
	author: ActorId,
	thread: ThreadId,
	text: impl Into<String>,
) -> Post {
	Post::new_raw(
		author,
		thread,
		PostIntent::REASONING_SUMMARY,
		MediaType::Text,
		text.into().into_bytes(),
		serde_json::Map::new(),
	)
}


// ═══════════════════════════════════════════════════════════════════════
// Post convenience methods
// ═══════════════════════════════════════════════════════════════════════

impl Post {
	/// Reads [`PostStatus`] from metadata.
	pub fn status(&self) -> PostStatus { post_status(self) }

	/// Writes [`PostStatus`] into metadata.
	pub fn set_status(&mut self, status: PostStatus) {
		set_post_status(self, status)
	}

	/// Classifies this post into a typed [`AgentPost`] view.
	pub fn as_agent_post(&self) -> AgentPost<'_> { AgentPost::new(self) }

	/// Returns `true` if this is a text post (OK intent, text-like media).
	pub fn is_text(&self) -> bool {
		self.intent() == PostIntent::OK && self.media_type().is_text()
	}

	/// Returns `true` if this is a refusal post.
	pub fn is_refusal(&self) -> bool { self.intent() == PostIntent::REFUSAL }

	/// Returns `true` if this is a reasoning content post.
	pub fn is_reasoning_content(&self) -> bool {
		self.intent() == PostIntent::REASONING_CONTENT
	}

	/// Returns `true` if this is a reasoning summary post.
	pub fn is_reasoning_summary(&self) -> bool {
		self.intent() == PostIntent::REASONING_SUMMARY
	}

	/// Returns `true` if this is a URL post.
	pub fn is_url(&self) -> bool { self.media_type() == &MediaType::Url }

	/// Returns `true` if this is a function call post.
	pub fn is_function_call(&self) -> bool {
		FunctionCallView::try_new(self).is_some()
	}

	/// Returns `true` if this is a function call output post.
	pub fn is_function_call_output(&self) -> bool {
		FunctionCallOutputView::try_new(self).is_some()
	}

	/// Returns a [`FunctionCallView`] if this is a function call post.
	pub fn as_function_call(&self) -> Option<FunctionCallView<'_>> {
		FunctionCallView::try_new(self)
	}

	/// Returns a [`FunctionCallOutputView`] if this is a function call
	/// output post.
	pub fn as_function_call_output(
		&self,
	) -> Option<FunctionCallOutputView<'_>> {
		FunctionCallOutputView::try_new(self)
	}

	/// Returns the URL string if this is a URL post.
	pub fn as_url_str(&self) -> Option<&str> {
		if self.media_type() == &MediaType::Url {
			self.as_str().ok()
		} else {
			None
		}
	}

	/// Constructs a filename from metadata `file_stem` and the media type
	/// extension.
	pub fn filename(&self) -> Option<String> {
		let file_stem = self
			.metadata()
			.get("file_stem")
			.and_then(|val| val.as_str())
			.unwrap_or("file");
		if let Some(ext) = self.media_type().extension() {
			Some(format!("{file_stem}.{ext}"))
		} else {
			Some(file_stem.to_string())
		}
	}

}
