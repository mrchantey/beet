//! A secrets document over any [`BlobStore`].

use crate::prelude::*;
use beet_core::prelude::*;

/// A secrets document at a path in a store: the file behind every `secrets`
/// verb and the `<Secrets>` load, read and written typed in the format its
/// extension names. The index is plaintext, so a read needs no identity;
/// [`SecretsDocument::open`] takes one.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let mut identities = AgeIdentityFile::default();
/// identities.push(AgeIdentity::generate());
/// let handle = SecretsHandle::new(BlobStore::temp(), "secrets.toml")?;
/// let mut document = handle.read_or_new().await?;
/// document.set(&identities, "TOKEN", "hunter2", default())?;
/// handle.write(&document).await?;
/// handle
/// 	.read()
/// 	.await?
/// 	.open(&identities)?
/// 	.get("TOKEN")
/// 	.unwrap()
/// 	.value
/// 	.as_str()
/// 	.xpect_eq("hunter2");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SecretsHandle {
	/// The store the document lives in.
	pub store: BlobStore,
	/// The document within the store, its extension naming the format.
	pub path: RelPath,
	/// The `<Secrets label>` this handle resolved from, for messages.
	pub label: Option<SmolStr>,
}

impl SecretsHandle {
	/// The document at `path` in `store`. Errors on a path naming no
	/// document format.
	pub fn new(store: BlobStore, path: impl AsRef<str>) -> Result<Self> {
		SecretsDocument::media_type_of(path.as_ref())?;
		Self {
			store,
			path: RelPath::new(path),
			label: None,
		}
		.xok()
	}

	/// The document a filesystem path or store uri names, see
	/// [`BlobStore::from_file_uri`]: `~/personal.toml`,
	/// `s3://bucket/secrets/x.toml`, or a bare `x.toml` in the cwd.
	pub fn from_uri(path: &str) -> Result<Self> {
		let (store, file) = BlobStore::from_file_uri(path)?;
		Self::new(store, file)
	}

	/// The handle with the label it was declared under.
	pub fn with_label(mut self, label: impl Into<SmolStr>) -> Self {
		self.label = Some(label.into());
		self
	}

	/// How a log or an error names this document: its label and path when
	/// declared, its path otherwise.
	pub fn describe(&self) -> String {
		match &self.label {
			Some(label) => format!("`{label}` ({})", self.path),
			None => format!("`{}`", self.path),
		}
	}

	/// The format the path names.
	pub fn media_type(&self) -> Result<MediaType> {
		SecretsDocument::media_type_of(self.path.as_str())
	}

	/// Whether the document exists in its store.
	pub async fn exists(&self) -> Result<bool> {
		self.store.exists(&self.path).await
	}

	/// Read and parse the document. Errors when the file is missing.
	pub async fn read(&self) -> Result<SecretsDocument> {
		let bytes =
			self.store.get(&self.path).await.map_err(|err| match err
				.downcast_ref::<HttpError>()
			{
				Some(err) if err.status_code == StatusCode::NOT_FOUND => {
					bevyhow!(
						"document {} is not written yet (`secrets/set` writes a \
						document, `<SecretsExport>` an export)",
						self.describe()
					)
				}
				_ => bevyhow!(
					"document {} cannot be read: {err}",
					self.describe()
				),
			})?;
		SecretsDocument::parse(self.media_type()?, &bytes)
			.map_err(|err| bevyhow!("document {}: {err}", self.describe()))
	}

	/// [`read`](Self::read), or an empty document when the file does not
	/// exist yet: the state a first `set` starts from.
	pub async fn read_or_new(&self) -> Result<SecretsDocument> {
		match self.exists().await? {
			true => self.read().await,
			false => SecretsDocument::new(self.media_type()?).xok(),
		}
	}

	/// Write `document` to this path in its format.
	pub async fn write(&self, document: &SecretsDocument) -> Result<()> {
		self.store.insert(&self.path, document.to_bytes()?).await
	}

	/// The document of a dated series beside this path, taken at `now`:
	/// `<dir>/YYYY/MM/DD/HHMMSSZ.<ext>`, the shape a snapshot series has so
	/// one listing reads either. An export with `dated=true` writes here; the
	/// declared path itself is never written.
	pub fn dated(&self, now: Timestamp) -> Result<Self> {
		let (year, month, day) = now.civil_date();
		let secs = now.secs().rem_euclid(86_400);
		let path = format!(
			"{}{year:04}/{month:02}/{day:02}/{:02}{:02}{:02}Z.{}",
			self.dir_prefix(),
			secs / 3600,
			secs % 3600 / 60,
			secs % 60,
			self.extension()?
		);
		Self::new(self.store.clone(), path)?
			.with_label_of(self)
			.xok()
	}

	/// The newest document of the dated series beside this path, `None`
	/// when none has been written: the one a restore and a probe read.
	/// Descends year, month and day newest first by one directory listing
	/// each, so a document at a repo root never walks the repo.
	pub async fn newest_dated(&self) -> Result<Option<Self>> {
		let suffix = SmolStr::new(format!("Z.{}", self.extension()?));
		Self::newest_in(
			self.store.clone(),
			RelPath::new(self.dir_prefix()),
			&[4, 2, 2],
			suffix,
		)
		.await?
		.map(|path| {
			Self::new(self.store.clone(), path.as_str())
				.map(|handle| handle.with_label_of(self))
		})
		.transpose()
	}

	/// The newest dated file under `dir`: at each level the digit
	/// directories of the next `width`, newest first, backtracking past one
	/// holding no series; at the leaf the newest `HHMMSSZ.<ext>` file.
	fn newest_in(
		store: BlobStore,
		dir: RelPath,
		widths: &'static [usize],
		suffix: SmolStr,
	) -> SendBoxedFuture<Result<Option<RelPath>>> {
		Box::pin(async move {
			let listing = store.list_dir(&dir).await?;
			let Some((&width, rest)) = widths.split_first() else {
				return listing
					.files
					.into_iter()
					.filter(|name| {
						name.strip_suffix(suffix.as_str())
							.is_some_and(|clock| Self::is_digits(clock, 6))
					})
					.max()
					.map(|name| dir.join(name))
					.xok();
			};
			for name in listing
				.dirs
				.into_iter()
				.filter(|name| Self::is_digits(name, width))
				.rev()
			{
				let found = Self::newest_in(
					store.clone(),
					dir.join(name),
					rest,
					suffix.clone(),
				)
				.await?;
				if found.is_some() {
					return Ok(found);
				}
			}
			Ok(None)
		})
	}

	/// Whether `path` (relative to the series dir) has the dated shape,
	/// `YYYY/MM/DD/HHMMSSZ.<extension>`.
	pub fn is_dated(path: &str, extension: &str) -> bool {
		let Some(stem) = path.strip_suffix(&format!("Z.{extension}")) else {
			return false;
		};
		let parts = stem.split('/').collect::<Vec<_>>();
		matches!(parts.as_slice(), [year, month, day, clock]
			if Self::is_digits(year, 4) && Self::is_digits(month, 2)
			&& Self::is_digits(day, 2) && Self::is_digits(clock, 6))
	}

	/// Exactly `width` ascii digits.
	fn is_digits(text: &str, width: usize) -> bool {
		text.len() == width && text.chars().all(|char| char.is_ascii_digit())
	}

	/// The directory the path sits in, with its trailing slash, empty at
	/// the store root.
	fn dir_prefix(&self) -> String {
		self.path
			.as_str()
			.rsplit_once('/')
			.map(|(dir, _)| format!("{dir}/"))
			.unwrap_or_default()
	}

	/// The path's extension, which names the format.
	fn extension(&self) -> Result<&str> {
		self.path
			.as_str()
			.rsplit_once('.')
			.map(|(_, extension)| extension)
			.ok_or_else(|| bevyhow!("`{}` has no extension", self.path))
	}

	fn with_label_of(self, other: &Self) -> Self {
		Self {
			label: other.label.clone(),
			..self
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn reads_and_writes_a_document() {
		let mut identities = AgeIdentityFile::default();
		identities.push(AgeIdentity::generate());
		let store = BlobStore::temp();
		let handle = SecretsHandle::new(store.clone(), "secrets.toml").unwrap();
		handle.exists().await.unwrap().xpect_false();
		handle
			.read()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("not written yet");
		let mut document = handle.read_or_new().await.unwrap();
		document.media_type().xpect_eq(MediaType::Toml);
		document.set(&identities, "A", "1", default()).unwrap();
		handle.write(&document).await.unwrap();
		// the index is plaintext toml
		String::from_utf8(
			store
				.get(&RelPath::new("secrets.toml"))
				.await
				.unwrap()
				.to_vec(),
		)
		.unwrap()
		.xpect_contains("[secrets.A]");
		handle
			.read()
			.await
			.unwrap()
			.open(&identities)
			.unwrap()
			.get("A")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("1");
		SecretsHandle::new(store, "cert.pem.age")
			.unwrap_err()
			.to_string()
			.xpect_contains("not a secrets document");
	}

	/// A dated series sits beside the declared path in the snapshot shape,
	/// and the newest is read back by name.
	#[beet_core::test]
	async fn dates_a_series_beside_the_path() {
		let store = BlobStore::temp();
		let handle = SecretsHandle::new(store.clone(), "secrets/export.toml")
			.unwrap()
			.with_label("cold");
		let midnight = Timestamp::parse_date("2026-09-15").unwrap();
		let first = handle.dated(midnight).unwrap();
		first
			.path
			.as_str()
			.xpect_eq("secrets/2026/09/15/000000Z.toml");
		first.label.clone().unwrap().as_str().xpect_eq("cold");
		let later = handle
			.dated(Timestamp::from_secs(midnight.secs() + 3661))
			.unwrap();
		later
			.path
			.as_str()
			.xpect_eq("secrets/2026/09/15/010101Z.toml");
		handle.newest_dated().await.unwrap().xpect_none();
		let document = SecretsDocument::default();
		first.write(&document).await.unwrap();
		later.write(&document).await.unwrap();
		// a stray file beside the series is not one of it
		store
			.insert(&RelPath::new("secrets/notes.toml"), "x")
			.await
			.unwrap();
		handle
			.newest_dated()
			.await
			.unwrap()
			.unwrap()
			.path
			.as_str()
			.xpect_eq("secrets/2026/09/15/010101Z.toml");
		// at the store root the series is bare
		SecretsHandle::new(store, "x.json")
			.unwrap()
			.dated(midnight)
			.unwrap()
			.path
			.as_str()
			.xpect_eq("2026/09/15/000000Z.json");
	}

	/// A document at a store root (the repo store's `secrets.toml`) looks
	/// for its series by descending dated directories, so a sibling tree
	/// (`target/`) is never walked and never mistaken for one.
	#[beet_core::test]
	async fn finds_a_root_series_among_siblings() {
		let store = BlobStore::temp();
		let handle = SecretsHandle::new(store.clone(), "secrets.toml").unwrap();
		for path in [
			"target/debug/deps/x.rlib",
			"2026/09/notes.toml",
			"2026/09/15/badclock.toml",
			"2025/12/31/235959Z.toml",
		] {
			store.insert(&RelPath::new(path), "x").await.unwrap();
		}
		handle
			.newest_dated()
			.await
			.unwrap()
			.unwrap()
			.path
			.as_str()
			.xpect_eq("2025/12/31/235959Z.toml");
		store
			.insert(&RelPath::new("2026/09/15/010101Z.toml"), "x")
			.await
			.unwrap();
		handle
			.newest_dated()
			.await
			.unwrap()
			.unwrap()
			.path
			.as_str()
			.xpect_eq("2026/09/15/010101Z.toml");
	}

	#[beet_core::test]
	fn resolves_a_path_or_uri() {
		let handle =
			SecretsHandle::from_uri("memory://documents/dir/x.json").unwrap();
		handle.path.as_str().xpect_eq("x.json");
		handle.store.subdir().as_str().xpect_eq("dir");
		handle.describe().xpect_eq("`x.json`");
		SecretsHandle::from_uri("dir/x.age").unwrap_err();
	}
}
