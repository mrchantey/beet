//! The files a post holds by reference, see [`PostFile`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;

/// A file a post holds by reference: a `media` post's one file, or one of
/// the files a tool answered ([`AgentPost::new_function_call_output_files`]),
/// named by its post and its index there. Its bytes live at `blobs/<cid>` in
/// the nearest blob store, or in memory on the post until they are written
/// (the model page, [`crate::table`], "Bytes").
#[derive(Debug, Clone, PartialEq)]
pub struct PostFile {
	/// The post holding the file.
	post: PostId,
	/// Where the file is among its post's files.
	index: usize,
	/// The reference to the file's bytes.
	blob: BlobRef,
	/// The file's name without its extension.
	file_stem: Option<SmolStr>,
}

/// A file by its post and its index there ([`PostFile::key`]).
pub type FileKey = (PostId, usize);

impl PostFile {
	/// The files `post` holds, in order: a `media` post's `blob`, else each
	/// of its `files`.
	pub fn of(post: &Post) -> Vec<Self> {
		let file = |index: usize, body: &Map| -> Option<Self> {
			Self {
				post: post.id(),
				index,
				blob: body.get("blob").ok()?.clone().into_serde().ok()?,
				file_stem: body
					.get("file_stem")
					.ok()
					.and_then(|stem| stem.as_str().ok())
					.map(SmolStr::from),
			}
			.xsome()
		};
		match post
			.body()
			.get("files")
			.ok()
			.and_then(|files| files.as_list().ok())
		{
			Some(files) => files
				.iter()
				.enumerate()
				.filter_map(|(index, entry)| file(index, entry.as_map().ok()?))
				.collect(),
			None => file(0, post.body()).into_iter().collect(),
		}
	}

	/// The newest `keep` of `files`, given oldest first: the ones a context
	/// keeping `keep` carries ([`KeepBlobs`]).
	pub fn newest(
		files: impl DoubleEndedIterator<Item = Self>,
		keep: usize,
	) -> HashSet<FileKey> {
		files.rev().take(keep).map(|file| file.key()).collect()
	}

	/// The post holding the file.
	pub fn post(&self) -> PostId { self.post }
	/// Where the file is among its post's files.
	pub fn index(&self) -> usize { self.index }
	/// The file by its post and index.
	pub fn key(&self) -> FileKey { (self.post, self.index) }
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
	/// ([`KeepBlobs`]), naming it by its post and its index there, counted
	/// from one, ie `[file: cat.png, image/png, 3 KB] file 2 in post 3mwj..`.
	pub fn listing(&self) -> String {
		format!(
			"{} file {} in post {}",
			self.line(),
			self.index + 1,
			self.post
		)
	}
}

impl Post {
	/// The files this post holds by reference, in order ([`PostFile`]).
	pub fn files(&self) -> Vec<PostFile> { PostFile::of(self) }

	/// This `media` post holding `bytes` as its one file: its body the
	/// [`BlobRef`] and stem, the bytes held in memory until they are written
	/// to the blob store.
	pub(crate) fn with_file(
		mut self,
		media_type: MediaType,
		bytes: Bytes,
		file_stem: Option<String>,
	) -> Self {
		let entry = Self::file_entry(&media_type, &bytes, file_stem);
		self.body_mut().extend(entry);
		self.set_bytes(vec![bytes]);
		self
	}

	/// This post holding `files`, each a [`MediaBytes`] and its stem, as its
	/// body's `files`, the bytes held in memory until they are written.
	pub(crate) fn with_files(
		mut self,
		files: impl IntoIterator<Item = (MediaBytes, Option<String>)>,
	) -> Self {
		let (entries, bytes): (Vec<Value>, Vec<Bytes>) = files
			.into_iter()
			.map(|(media, file_stem)| {
				let (media_type, bytes) = media.take();
				let entry = Self::file_entry(&media_type, &bytes, file_stem);
				(Value::Map(entry), bytes)
			})
			.unzip();
		if !entries.is_empty() {
			self.body_mut().insert("files", Value::List(entries));
			self.set_bytes(bytes);
		}
		self
	}

	/// The body entry of one file, `{blob, file_stem?}`.
	fn file_entry(
		media_type: &MediaType,
		bytes: &Bytes,
		file_stem: Option<String>,
	) -> Map {
		let mut entry = Map::default();
		// a blob reference always serializes
		if let Ok(blob) =
			Value::from_serde(&BlobRef::of(bytes, media_type.clone()))
		{
			entry.insert("blob", blob);
		}
		if let Some(stem) = file_stem {
			entry.insert("file_stem", stem);
		}
		entry
	}
}
