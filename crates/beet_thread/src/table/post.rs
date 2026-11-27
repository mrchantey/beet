use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;
use beet_net::prelude::*;

/// A post by an actor on a thread: one row of its thread's `post` table, a
/// small envelope every kind shares and one body per kind (the model page,
/// [`crate::table`], "Posts").
///
/// Its id carries its time and its partition is its thread, so the row
/// stores neither: [`Self::thread`] is set by whatever loaded it, since a
/// branch's records hold several threads' posts. The bytes of a file a post
/// holds ([`Self::file`]) are never in its row: they ride on the post in
/// memory ([`Self::bytes`]) until a flush writes them to the blob store, and
/// are read back for a request.
///
/// Read a post's content through its typed view ([`AgentPost`]), a log's
/// through its kind ([`Self::log`]). Posts order by id.
#[derive(Clone, PartialEq, Serialize, Deserialize, Reflect, Component)]
#[reflect(Serialize, Deserialize, Component)]
pub struct Post {
	/// The row's key, never in its body.
	#[serde(skip)]
	id: PostId,
	/// The thread whose partition holds the row, set on load.
	#[serde(skip)]
	thread: ThreadId,
	/// The actor that authored this post.
	author: ActorId,
	/// What the post is.
	kind: PostKind,
	#[serde(default)]
	status: PostStatus,
	/// The post this one is about: the call an output answers, the post a
	/// log edits or deletes.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	subject: Option<PostId>,
	/// The kind's own fields.
	#[serde(default)]
	body: Map,
	/// The bytes of the file the post holds, in memory and never serialized.
	#[serde(skip)]
	#[reflect(ignore)]
	bytes: Option<Bytes>,
}

impl std::fmt::Debug for Post {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Post")
			.field("id", &self.id)
			.field("thread", &self.thread)
			.field("author", &self.author)
			.field("kind", &self.kind)
			.field("status", &self.status)
			.field("subject", &self.subject)
			.field("body", &self.body)
			.field("bytes", &self.bytes.as_ref().map(|bytes| bytes.len()))
			.finish()
	}
}

impl PartialOrd for Post {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.id.cmp(&other.id))
	}
}

impl TableStoreRow for Post {
	fn table_name() -> SmolStr { "post".into() }
	fn key(&self) -> TableKey { self.id.into() }
	fn indexes() -> Vec<TableIndex> {
		vec![
			TableIndex::new("by_subject", ["subject"]),
			TableIndex::new("by_kind", ["kind", "author"]),
		]
	}
	fn from_row(key: &TableKey, row: Row) -> Result<Self> {
		let mut post = row.into_serde::<Self>()?;
		post.id = PostId::parse(key)?;
		post.xok()
	}
}

// ═══════════════════════════════════════════════════════════════════════
// PostKind
// ═══════════════════════════════════════════════════════════════════════

/// What a post is: the closed list of kinds beet defines, and any other a
/// log of another crate, its name prefixing that crate ([`LogKind`]).
///
/// Written as its snake case name, ie `tool_output`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(opaque)]
#[reflect(Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub enum PostKind {
	/// `{text}`: something said.
	#[default]
	Text,
	/// `{url, media_type, file_stem?}`: a link to a file elsewhere.
	Url,
	/// `{blob, file_stem?}`: a file, its bytes by reference.
	Media,
	/// `{text}`: a model refused.
	Refusal,
	/// `{text}`: a response failed.
	Error,
	/// `{text}`: a model's reasoning.
	Reasoning,
	/// `{text}`: a summary of a model's reasoning.
	ReasoningSummary,
	/// `{name, call_id, arguments}`: a tool called.
	ToolCall,
	/// `{call_id, text, blob?, file_stem?}`, its subject the call: what a
	/// tool answered, its bytes by reference.
	ToolOutput,
	/// A log: an actor joined ([`SessionOpened`]).
	SessionOpened,
	/// A log: a post was edited, its subject the post ([`PostEdited`]).
	Edited,
	/// A log: a post was deleted, its subject the post ([`PostDeleted`]).
	Deleted,
	/// A log: a summary of everything up to its subject.
	Compaction,
	/// A log of another crate, ie `beet_eval/decision`.
	Other(SmolStr),
}

impl PostKind {
	/// The kind's written name.
	pub fn as_str(&self) -> &str {
		match self {
			Self::Text => "text",
			Self::Url => "url",
			Self::Media => "media",
			Self::Refusal => "refusal",
			Self::Error => "error",
			Self::Reasoning => "reasoning",
			Self::ReasoningSummary => "reasoning_summary",
			Self::ToolCall => "tool_call",
			Self::ToolOutput => "tool_output",
			Self::SessionOpened => "session_opened",
			Self::Edited => "edited",
			Self::Deleted => "deleted",
			Self::Compaction => "compaction",
			Self::Other(name) => name,
		}
	}

	/// The kind a name writes, any name beet does not define being a log of
	/// another crate.
	pub fn parse(name: &str) -> Self {
		match name {
			"text" => Self::Text,
			"url" => Self::Url,
			"media" => Self::Media,
			"refusal" => Self::Refusal,
			"error" => Self::Error,
			"reasoning" => Self::Reasoning,
			"reasoning_summary" => Self::ReasoningSummary,
			"tool_call" => Self::ToolCall,
			"tool_output" => Self::ToolOutput,
			"session_opened" => Self::SessionOpened,
			"edited" => Self::Edited,
			"deleted" => Self::Deleted,
			"compaction" => Self::Compaction,
			other => Self::Other(other.into()),
		}
	}

	/// Whether a post of this kind is a log, what happened to the thread
	/// rather than what was said in it.
	pub fn is_log(&self) -> bool {
		matches!(
			self,
			Self::SessionOpened
				| Self::Edited
				| Self::Deleted
				| Self::Compaction
				| Self::Other(_)
		)
	}

	/// Whether a model reads a post of this kind as a note: a join, an edit
	/// or a deletion ([`ThreadRecords::note`]).
	pub fn is_noted(&self) -> bool {
		matches!(self, Self::SessionOpened | Self::Edited | Self::Deleted)
	}

	/// The body field a post of this kind streams into, `None` for a kind
	/// that never streams.
	pub fn text_field(&self) -> Option<&'static str> {
		match self {
			Self::Text
			| Self::Refusal
			| Self::Error
			| Self::Reasoning
			| Self::ReasoningSummary
			| Self::ToolOutput
			| Self::Compaction => Some("text"),
			Self::ToolCall => Some("arguments"),
			Self::Url => Some("url"),
			_ => None,
		}
	}

	/// Teach every authoring seam to read a kind from its name, ie
	/// `<CreatePost kind="refusal"/>`.
	pub(crate) fn register_literal() {
		LiteralParser::register::<Self>(LiteralParser::new(|value: &Value| {
			match value {
				Value::Str(text) => Self::parse(text).xsome().xok(),
				_ => Ok(None),
			}
		}));
	}
}

impl std::fmt::Display for PostKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

impl Serialize for PostKind {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(self.as_str())
	}
}

impl<'de> Deserialize<'de> for PostKind {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		SmolStr::deserialize(deserializer).map(|name| Self::parse(&name))
	}
}

// ═══════════════════════════════════════════════════════════════════════
// PostStatus
// ═══════════════════════════════════════════════════════════════════════

/// Whether a post is complete, in progress, closed by an interrupt, or
/// deleted by its author, a tombstone whose row keeps its body (the model
/// page, [`crate::table`], "Editing and deleting").
#[derive(
	Debug,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
	#[default]
	Completed,
	Interrupted,
	InProgress,
	Deleted,
}

impl PostStatus {
	/// The status's written name, ie `in_progress`.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Completed => "completed",
			Self::Interrupted => "interrupted",
			Self::InProgress => "in_progress",
			Self::Deleted => "deleted",
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Accessors
// ═══════════════════════════════════════════════════════════════════════

impl Post {
	/// A post of `kind` by `author` on `thread`, its fields `body`, minted
	/// now. Prefer the [`AgentPost`] constructors, ie
	/// [`AgentPost::new_text`].
	pub fn new(
		author: ActorId,
		thread: ThreadId,
		kind: PostKind,
		status: PostStatus,
		body: Map,
	) -> Self {
		Self {
			id: PostId::now(),
			thread,
			author,
			kind,
			status,
			subject: None,
			body,
			bytes: None,
		}
	}

	pub fn id(&self) -> PostId { self.id }
	pub fn author(&self) -> &ActorId { &self.author }
	pub fn thread(&self) -> ThreadId { self.thread }
	pub fn kind(&self) -> &PostKind { &self.kind }
	pub fn status(&self) -> PostStatus { self.status }
	pub fn subject(&self) -> Option<PostId> { self.subject }
	pub fn body(&self) -> &Map { &self.body }
	/// The instant the post was created, its id's.
	pub fn created(&self) -> Timestamp { self.id.created() }

	/// Whether this post is a log ([`PostKind::is_log`]).
	pub fn is_log(&self) -> bool { self.kind.is_log() }

	/// Whether this post is something someone said, a candidate for the last
	/// word ([`ThreadRecords::last_word`]): neither a log nor a
	/// `tool_output` closing an interrupted call, which nobody said.
	pub fn is_word(&self) -> bool {
		let closed_call = self.status == PostStatus::Interrupted
			&& self.kind == PostKind::ToolOutput;
		!self.is_log() && !closed_call
	}

	/// Whether the post is still streaming.
	pub fn in_progress(&self) -> bool { self.status == PostStatus::InProgress }

	/// Whether the post's author deleted it.
	pub fn is_deleted(&self) -> bool { self.status == PostStatus::Deleted }

	pub fn set_status(&mut self, status: PostStatus) { self.status = status; }

	/// Name the thread whose partition holds this post, as it loads.
	pub(crate) fn set_thread(&mut self, thread: ThreadId) {
		self.thread = thread;
	}

	/// This post about `subject`.
	pub fn with_subject(mut self, subject: PostId) -> Self {
		self.subject = Some(subject);
		self
	}

	/// The string field `name` of the body.
	pub fn field(&self, name: &str) -> Option<&str> {
		self.body.get(name).ok()?.as_str().ok()
	}

	/// The text the post's kind streams into ([`PostKind::text_field`]).
	pub fn text(&self) -> Option<&str> { self.field(self.kind.text_field()?) }

	/// Append to the text the post's kind streams into.
	pub fn push_str(&mut self, text: &str) {
		let mut joined = self.text().unwrap_or_default().to_string();
		joined.push_str(text);
		self.set_text(joined);
	}

	/// Replace the text the post's kind streams into.
	pub fn set_text(&mut self, text: impl Into<String>) {
		if let Some(field) = self.kind.text_field() {
			self.body.insert(field, text.into());
		}
	}

	/// Set the string field `name` of the body.
	pub(crate) fn set_field(&mut self, name: &str, value: impl Into<String>) {
		self.body.insert(name, value.into());
	}

	/// The body, mutably.
	pub(crate) fn body_mut(&mut self) -> &mut Map { &mut self.body }

	/// Replace the body, as an edit does.
	pub(crate) fn set_body(&mut self, body: Map) { self.body = body; }

	/// A copy with its body and bytes emptied, as shown in place of what was
	/// taken back ([`ShownPosts`]).
	pub(crate) fn emptied(&self) -> Self {
		Self {
			body: Map::default(),
			bytes: None,
			..self.clone()
		}
	}

	/// The bytes of the file this post holds ([`Self::file`]), while it
	/// holds them in memory.
	pub fn bytes(&self) -> Option<&Bytes> { self.bytes.as_ref() }

	/// Hold `bytes` in memory on this post.
	pub(crate) fn set_bytes(&mut self, bytes: Bytes) {
		self.bytes = Some(bytes);
	}

	/// Drop the bytes held in memory, which the blob store, if any, holds.
	pub(crate) fn release_bytes(&mut self) { self.bytes = None; }
}

impl std::fmt::Display for Post {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(text) = self.text() {
			return f.write_str(text);
		}
		match self.file() {
			Some(file) => {
				write!(f, "[{}, {} bytes]", file.media_type(), file.size())
			}
			None => write!(f, "[{}]", self.kind),
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Seeds
// ═══════════════════════════════════════════════════════════════════════

impl Post {
	/// Author a seed post in rust as a child of its [`Actor`], ie
	/// `children![Post::spawn("hello")]`, as `<CreatePost>` does in markup. The
	/// author-to-behavior reduction resolves its author (the actor parent) and
	/// thread, hoists it into the [`ThreadRecords`] and despawns the entity,
	/// leaving only the record. An entry continuing a thread discards it unread.
	///
	/// Its kind follows the content; see [`Self::spawn_with`] /
	/// `<CreatePost>` to author refusals, reasoning, or in-progress posts.
	pub fn spawn(content: impl Into<IntoPost>) -> SeedPost {
		SeedPost {
			content: content.into(),
			kind: None,
			status: PostStatus::Completed,
		}
	}

	/// As [`Self::spawn`], text of `kind` with `status`. This is the
	/// resolver `<CreatePost>` wraps.
	pub fn spawn_with(
		content: impl Into<IntoPost>,
		kind: PostKind,
		status: PostStatus,
	) -> SeedPost {
		SeedPost {
			content: content.into(),
			kind: Some(kind),
			status,
		}
	}
}

/// Content for constructing a new [`Post`].
///
/// Used by spawners like [`Post::spawn`] (via [`SeedPost`]).
/// Prefer [`AgentPost`] constructors for full control over kind and status.
#[derive(Debug, Clone)]
pub enum IntoPost {
	Text(String),
	Url {
		url: String,
		media_type: MediaType,
		file_stem: Option<String>,
	},
	Bytes {
		media_type: MediaType,
		bytes: Vec<u8>,
		file_stem: Option<String>,
	},
	FunctionCall {
		name: String,
		call_id: String,
		arguments: String,
	},
	FunctionCallOutput {
		call: PostId,
		call_id: String,
		output: String,
	},
}

impl IntoPost {
	/// Converts into a [`Post`] with the given author, thread and status.
	pub fn into_post(
		self,
		author: ActorId,
		thread: ThreadId,
		status: PostStatus,
	) -> Post {
		match self {
			IntoPost::Text(text) => {
				AgentPost::new_text(author, thread, text, status)
			}
			IntoPost::Url {
				url,
				media_type,
				file_stem,
			} => AgentPost::new_url(
				author, thread, url, media_type, file_stem, status,
			),
			IntoPost::Bytes {
				media_type,
				bytes,
				file_stem,
			} => AgentPost::new_media(
				author, thread, media_type, bytes, file_stem, status,
			),
			IntoPost::FunctionCall {
				name,
				call_id,
				arguments,
			} => AgentPost::new_function_call(
				author, thread, name, call_id, arguments, status,
			),
			IntoPost::FunctionCallOutput {
				call,
				call_id,
				output,
			} => AgentPost::new_function_call_output(
				author, thread, call, call_id, output, status,
			),
		}
	}
}

impl From<String> for IntoPost {
	fn from(text: String) -> Self { IntoPost::Text(text) }
}

impl From<&str> for IntoPost {
	fn from(text: &str) -> Self { IntoPost::Text(text.to_string()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The row is the envelope alone: neither the id, the thread nor any
	/// bytes, and the kind its name.
	#[beet_core::test]
	fn a_row_is_the_envelope() {
		let thread = ThreadId::now();
		let call = AgentPost::new_function_call(
			ActorId::AGENT,
			thread,
			"search",
			"c1",
			"{}",
			PostStatus::Completed,
		);
		let rows = [
			AgentPost::new_text(
				ActorId::USER,
				thread,
				"hi",
				PostStatus::Completed,
			),
			AgentPost::new_url(
				ActorId::USER,
				thread,
				"https://example.com/cat.png",
				MediaType::Png,
				Some("cat".into()),
				PostStatus::Completed,
			),
			AgentPost::new_media(
				ActorId::USER,
				thread,
				MediaType::Png,
				vec![1, 2, 3],
				None,
				PostStatus::Completed,
			),
			AgentPost::new_refusal(
				ActorId::AGENT,
				thread,
				"no",
				PostStatus::Completed,
			),
			AgentPost::new_error(
				ActorId::AGENT,
				thread,
				"failed",
				PostStatus::Interrupted,
			),
			AgentPost::new_reasoning(
				ActorId::AGENT,
				thread,
				"hmm",
				PostStatus::Completed,
			),
			AgentPost::new_reasoning_summary(
				ActorId::AGENT,
				thread,
				"thought",
				PostStatus::InProgress,
			),
			call.clone(),
			AgentPost::new_function_call_output(
				ActorId::AGENT,
				thread,
				call.id(),
				"c1",
				"found",
				PostStatus::Completed,
			),
			Post::new_log(ActorId::USER, thread, &SessionOpened::default())
				.unwrap(),
		]
		.into_iter()
		.map(|post| {
			post.into_row()
				.unwrap()
				.into_json_form()
				.to_json_string()
				// the subject is an id minted now
				.replace(&call.id().to_string(), "<call>")
		})
		.collect::<Vec<_>>()
		.join("\n");
		rows.xpect_snapshot();
	}

	/// A row reads back as the post it was, its id from the key.
	#[beet_core::test]
	fn reads_back_from_its_row() {
		let post = AgentPost::new_text(
			ActorId::USER,
			ThreadId::now(),
			"hi",
			PostStatus::Completed,
		);
		let mut read =
			Post::from_row(&post.key(), post.clone().into_row().unwrap())
				.unwrap();
		read.set_thread(post.thread());
		read.xpect_eq(post);
	}
}
