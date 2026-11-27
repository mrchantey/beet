use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;

/// A typed view of a [`Post`] that is not a log, whoever its author, by its
/// [`PostKind`]: the content a model reads or writes. A deleted post is
/// [`Self::Deleted`] whatever its kind, so no reader takes its body.
pub enum AgentPost<'a> {
	Text(TextView<'a>),
	Refusal(TextView<'a>),
	Url(UrlView<'a>),
	Media(MediaView<'a>),
	Error(TextView<'a>),
	FunctionCall(FunctionCallView<'a>),
	FunctionCallOutput(FunctionCallOutputView<'a>),
	ReasoningContent(TextView<'a>),
	ReasoningSummary(TextView<'a>),
	/// A post its author deleted, a tombstone with nothing to read.
	Deleted(DeletedView<'a>),
}

impl std::ops::Deref for AgentPost<'_> {
	type Target = Post;
	fn deref(&self) -> &Self::Target { self.post() }
}

impl<'a> AgentPost<'a> {
	/// The view of `post` by its kind, `None` for a log, which is not
	/// content.
	pub fn new(post: &'a Post) -> Option<Self> {
		let text = TextView { post };
		if post.is_deleted() && !post.is_log() {
			return Self::Deleted(DeletedView { post }).xsome();
		}
		match post.kind() {
			PostKind::Text => Self::Text(text),
			PostKind::Refusal => Self::Refusal(text),
			PostKind::Error => Self::Error(text),
			PostKind::Reasoning => Self::ReasoningContent(text),
			PostKind::ReasoningSummary => Self::ReasoningSummary(text),
			PostKind::Url => Self::Url(UrlView { post }),
			PostKind::Media => Self::Media(MediaView { post }),
			PostKind::ToolCall => Self::FunctionCall(FunctionCallView { post }),
			PostKind::ToolOutput => {
				Self::FunctionCallOutput(FunctionCallOutputView { post })
			}
			_ => return None,
		}
		.xsome()
	}

	/// Returns the underlying post reference.
	pub fn post(&self) -> &'a Post {
		match self {
			Self::Text(view)
			| Self::Refusal(view)
			| Self::Error(view)
			| Self::ReasoningContent(view)
			| Self::ReasoningSummary(view) => view.post,
			Self::Url(view) => view.post,
			Self::Media(view) => view.post,
			Self::FunctionCall(view) => view.post,
			Self::FunctionCallOutput(view) => view.post,
			Self::Deleted(view) => view.post,
		}
	}

	/// Whether a model reads this post: every kind but reasoning, which stays
	/// in the records and the view whoever wrote it, and nothing deleted.
	pub fn is_model_input(&self) -> bool {
		!matches!(
			self,
			Self::ReasoningContent(_)
				| Self::ReasoningSummary(_)
				| Self::Deleted(_)
		)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Post constructors
// ═══════════════════════════════════════════════════════════════════════

impl AgentPost<'static> {
	/// A post of `kind` whose body is `{text}`.
	fn new_text_kind(
		author: ActorId,
		thread: ThreadId,
		kind: PostKind,
		text: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Post::new(
			author,
			thread,
			kind,
			status,
			fields([("text", text.into())]),
		)
	}

	/// Something said.
	pub fn new_text(
		author: ActorId,
		thread: ThreadId,
		text: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Self::new_text_kind(author, thread, PostKind::Text, text, status)
	}

	/// A model's refusal.
	pub fn new_refusal(
		author: ActorId,
		thread: ThreadId,
		text: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Self::new_text_kind(author, thread, PostKind::Refusal, text, status)
	}

	/// A link to a file of `media_type` elsewhere.
	pub fn new_url(
		author: ActorId,
		thread: ThreadId,
		url: impl Into<String>,
		media_type: MediaType,
		file_stem: Option<String>,
		status: PostStatus,
	) -> Post {
		let mut body = fields([
			("url", url.into()),
			("media_type", media_type.as_str().to_string()),
		]);
		if let Some(stem) = file_stem {
			body.insert("file_stem", stem);
		}
		Post::new(author, thread, PostKind::Url, status, body)
	}

	/// A file: its row holds a [`BlobRef`] to `bytes`, which the post holds
	/// in memory until they are written to the blob store.
	pub fn new_media(
		author: ActorId,
		thread: ThreadId,
		media_type: MediaType,
		bytes: impl Into<Bytes>,
		file_stem: Option<String>,
		status: PostStatus,
	) -> Post {
		Post::new(author, thread, PostKind::Media, status, Map::default())
			.with_file(media_type, bytes.into(), file_stem)
	}

	/// A response's failure.
	pub fn new_error(
		author: ActorId,
		thread: ThreadId,
		message: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Self::new_text_kind(author, thread, PostKind::Error, message, status)
	}

	/// A model's reasoning.
	pub fn new_reasoning(
		author: ActorId,
		thread: ThreadId,
		text: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Self::new_text_kind(author, thread, PostKind::Reasoning, text, status)
	}

	/// A summary of a model's reasoning.
	pub fn new_reasoning_summary(
		author: ActorId,
		thread: ThreadId,
		text: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Self::new_text_kind(
			author,
			thread,
			PostKind::ReasoningSummary,
			text,
			status,
		)
	}

	/// A tool called with `arguments`, a json string.
	pub fn new_function_call(
		author: ActorId,
		thread: ThreadId,
		name: impl Into<String>,
		call_id: impl Into<String>,
		arguments: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Post::new(
			author,
			thread,
			PostKind::ToolCall,
			status,
			fields([
				("name", name.into()),
				("call_id", call_id.into()),
				("arguments", arguments.into()),
			]),
		)
	}

	/// What a tool answered the call `call`, its subject.
	pub fn new_function_call_output(
		author: ActorId,
		thread: ThreadId,
		call: PostId,
		call_id: impl Into<String>,
		output: impl Into<String>,
		status: PostStatus,
	) -> Post {
		Post::new(
			author,
			thread,
			PostKind::ToolOutput,
			status,
			fields([("call_id", call_id.into()), ("text", output.into())]),
		)
		.with_subject(call)
	}

	/// A tool's answer to the call `call` holding `files`, each a
	/// [`MediaBytes`] and its stem, by reference as a `media` post holds its
	/// one, beside the answer's `text`, which a reader not sent a file reads
	/// with each file's line ([`RequestBlobs::output_text`]).
	pub fn new_function_call_output_files(
		author: ActorId,
		thread: ThreadId,
		call: PostId,
		call_id: impl Into<String>,
		text: impl Into<String>,
		files: impl IntoIterator<Item = (MediaBytes, Option<String>)>,
		status: PostStatus,
	) -> Post {
		Self::new_function_call_output(
			author, thread, call, call_id, text, status,
		)
		.with_files(files)
	}
}

/// A body of string fields.
fn fields(pairs: impl IntoIterator<Item = (&'static str, String)>) -> Map {
	pairs
		.into_iter()
		.map(|(name, value)| (SmolStr::from(name), Value::from(value)))
		.collect()
}

// ═══════════════════════════════════════════════════════════════════════
// View types
// ═══════════════════════════════════════════════════════════════════════

/// A post its author deleted: its status and when it was made, and nothing
/// of what it said.
#[derive(Deref)]
pub struct DeletedView<'a> {
	post: &'a Post,
}

impl<'a> DeletedView<'a> {
	pub fn post(&self) -> &'a Post { self.post }
}

/// A post whose body is `{text}`: text, a refusal, an error or reasoning.
#[derive(Deref)]
pub struct TextView<'a> {
	post: &'a Post,
}

impl<'a> TextView<'a> {
	pub fn text(&self) -> &'a str {
		self.post.field("text").unwrap_or_default()
	}
	pub fn post(&self) -> &'a Post { self.post }
}

/// A link to a file elsewhere.
#[derive(Deref)]
pub struct UrlView<'a> {
	post: &'a Post,
}

impl<'a> UrlView<'a> {
	pub fn url(&self) -> &'a str { self.post.field("url").unwrap_or_default() }
	/// The linked file's media type.
	pub fn media_type(&self) -> MediaType {
		self.post
			.field("media_type")
			.map(MediaType::from)
			.unwrap_or_default()
	}
	pub fn file_stem(&self) -> Option<&'a str> { self.post.field("file_stem") }
	/// The stem and the media type's extension.
	pub fn filename(&self) -> String {
		PostFile::filename_of(self.file_stem(), &self.media_type())
	}
	pub fn post(&self) -> &'a Post { self.post }
}

/// A file, its bytes by reference.
#[derive(Deref)]
pub struct MediaView<'a> {
	post: &'a Post,
}

impl<'a> MediaView<'a> {
	/// The file the post holds, `None` for a malformed row.
	pub fn file(&self) -> Option<PostFile> {
		self.post.files().into_iter().next()
	}
	/// The file's media type.
	pub fn media_type(&self) -> MediaType {
		self.file()
			.map(|file| file.media_type())
			.unwrap_or_default()
	}
	pub fn post(&self) -> &'a Post { self.post }
}

/// A tool called.
#[derive(Deref)]
pub struct FunctionCallView<'a> {
	post: &'a Post,
}

impl<'a> FunctionCallView<'a> {
	/// The function name.
	pub fn name(&self) -> &'a str {
		self.post.field("name").unwrap_or_default()
	}
	/// The provider's identifier of the call.
	pub fn call_id(&self) -> &'a str {
		self.post.field("call_id").unwrap_or_default()
	}
	/// The arguments as a JSON string.
	pub fn arguments(&self) -> &'a str {
		self.post.field("arguments").unwrap_or_default()
	}
	pub fn post(&self) -> &'a Post { self.post }
	pub fn into_owned(self) -> OwnedFunctionCall {
		OwnedFunctionCall {
			post: self.post.id(),
			name: self.name().to_string(),
			call_id: self.call_id().to_string(),
			arguments: self.arguments().to_string(),
		}
	}
}

/// A call to execute, named by its post so its output can name it back.
pub struct OwnedFunctionCall {
	post: PostId,
	name: String,
	call_id: String,
	arguments: String,
}
impl OwnedFunctionCall {
	/// The call's post, its output's subject.
	pub fn post(&self) -> PostId { self.post }
	pub fn name(&self) -> &str { &self.name }
	pub fn call_id(&self) -> &str { &self.call_id }
	pub fn arguments(&self) -> &str { &self.arguments }
	/// The stem of a file the tool answers: the last segment of its route
	/// path, ie `take-photo`.
	pub fn file_stem(&self) -> Option<String> {
		FunctionToolDefinition::decode_name(&self.name)
			.rsplit('/')
			.find(|segment| !segment.is_empty())
			.map(str::to_string)
	}
}

/// What a tool answered.
#[derive(Deref)]
pub struct FunctionCallOutputView<'a> {
	post: &'a Post,
}

impl<'a> FunctionCallOutputView<'a> {
	/// The provider's identifier of the call answered.
	pub fn call_id(&self) -> &'a str {
		self.post.field("call_id").unwrap_or_default()
	}
	/// The call answered, the post's subject.
	pub fn call(&self) -> Option<PostId> { self.post.subject() }
	/// The answer's text, beside any files it holds.
	pub fn output(&self) -> &'a str {
		self.post.field("text").unwrap_or_default()
	}
	/// The files the tool answered.
	pub fn files(&self) -> Vec<PostFile> { self.post.files() }
	pub fn post(&self) -> &'a Post { self.post }
}

impl Post {
	/// Classifies this post into a typed [`AgentPost`] view, `None` for a
	/// log.
	pub fn as_agent_post(&self) -> Option<AgentPost<'_>> {
		AgentPost::new(self)
	}
}
