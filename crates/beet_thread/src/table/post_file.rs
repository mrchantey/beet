//! The file a post holds by reference, see [`PostFile`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;

/// A file a post holds by reference: a `media` post's, or a tool's answer
/// of bytes ([`AgentPost::new_function_call_output_file`]). Its bytes live at
/// `blobs/<cid>` in the nearest blob store, or in memory on the post until
/// they are written (the model page, [`crate::table`], "Bytes").
#[derive(Debug, Clone, PartialEq)]
pub struct PostFile {
	/// The post holding the file.
	post: PostId,
	/// The reference to the file's bytes.
	blob: BlobRef,
	/// The file's name without its extension.
	file_stem: Option<SmolStr>,
}

impl PostFile {
	/// The file `post` holds, `None` for a post holding none.
	pub fn of(post: &Post) -> Option<Self> {
		Self {
			post: post.id(),
			blob: post.body().get("blob").ok()?.clone().into_serde().ok()?,
			file_stem: post.field("file_stem").map(SmolStr::from),
		}
		.xsome()
	}

	/// The posts of the newest `keep` of `files`, given oldest first: the
	/// ones a context keeping `keep` carries ([`KeepBlobs`]).
	pub fn newest(
		files: impl DoubleEndedIterator<Item = Self>,
		keep: usize,
	) -> HashSet<PostId> {
		files.rev().take(keep).map(|file| file.post).collect()
	}

	/// The post holding the file.
	pub fn post(&self) -> PostId { self.post }
	/// The reference to the file's bytes.
	pub fn blob(&self) -> &BlobRef { &self.blob }
	pub fn media_type(&self) -> MediaType { self.blob.mime_type.clone() }
	/// The file's length in bytes.
	pub fn size(&self) -> u64 { self.blob.size }
	pub fn file_stem(&self) -> Option<&str> { self.file_stem.as_deref() }

	/// The stem and the media type's extension.
	pub fn filename(&self) -> String {
		Self::filename_of(self.file_stem(), &self.blob.mime_type)
	}

	/// A filename of `file_stem`, by default `file`, and the extension of
	/// `media_type`.
	pub fn filename_of(
		file_stem: Option<&str>,
		media_type: &MediaType,
	) -> String {
		let file_stem = file_stem.unwrap_or("file");
		match media_type.extension() {
			Some(ext) => format!("{file_stem}.{ext}"),
			None => file_stem.to_string(),
		}
	}

	/// The one line naming this file, what a reader that cannot carry it
	/// reads, ie `[file: report.pdf, application/pdf, 12 KB]`.
	pub fn line(&self) -> String {
		let size = match self.size() {
			size if size < 1024 => format!("{size} B"),
			size if size < 1024 * 1024 => format!("{} KB", size / 1024),
			size => format!("{} MB", size / (1024 * 1024)),
		};
		format!(
			"[file: {}, {}, {size}]",
			self.filename(),
			self.blob.mime_type
		)
	}

	/// The line of a file a context lists rather than carries
	/// ([`KeepBlobs`]), its post beside it, ie
	/// `[file: cat.png, image/png, 3 KB] in post 3mwj..`.
	pub fn listing(&self) -> String {
		format!("{} in post {}", self.line(), self.post)
	}
}

impl Post {
	/// The file this post holds by reference, if any ([`PostFile`]).
	pub fn file(&self) -> Option<PostFile> { PostFile::of(self) }

	/// This post holding `bytes` as a file of `media_type`: its body the
	/// [`BlobRef`] and stem, the bytes held in memory until they are written
	/// to the blob store.
	pub(crate) fn with_file(
		mut self,
		media_type: MediaType,
		bytes: Bytes,
		file_stem: Option<String>,
	) -> Self {
		let blob = BlobRef::of(&bytes, media_type);
		// a blob reference always serializes
		if let Ok(blob) = Value::from_serde(&blob) {
			self.body_mut().insert("blob", blob);
		}
		if let Some(stem) = file_stem {
			self.body_mut().insert("file_stem", stem);
		}
		self.set_bytes(bytes);
		self
	}
}
