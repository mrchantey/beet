//! Writing a compile into a package's space, each record only where its
//! content moved, and the [`CompileReport`] of what was written.
use super::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bytes::Bytes;

/// What one [`PackageCompiler::compile`] wrote into the package's space.
#[derive(Debug, Clone)]
pub struct CompileReport {
	/// The package compiled.
	pub package: Package,
	/// The branch compiled.
	pub branch: BranchName,
	/// The branch record as it stands.
	pub branch_record: StrongRef,
	/// The package record as it stands.
	pub package_record: StrongRef,
	/// The records created or rewritten.
	pub written: Vec<StrongRef>,
	/// The records already holding what this compile would write.
	pub unchanged: Vec<StrongRef>,
	/// The blobs uploaded.
	pub uploaded: Vec<Cid>,
	/// Each path dependency's own compile.
	pub dependencies: Vec<CompileReport>,
}

impl CompileReport {
	/// Whether this compile and its dependencies' wrote nothing, every
	/// record and blob already as compiled.
	pub fn wrote_nothing(&self) -> bool {
		self.written.is_empty()
			&& self.uploaded.is_empty()
			&& self.dependencies.iter().all(Self::wrote_nothing)
	}
}

/// One line per package: its name and branch, then what was written.
impl core::fmt::Display for CompileReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		for dependency in &self.dependencies {
			writeln!(f, "{dependency}")?;
		}
		write!(
			f,
			"compiled {}@{} ({}): {} written, {} unchanged, {} blobs uploaded",
			self.package.name,
			self.branch,
			self.package_record.uri,
			self.written.len(),
			self.unchanged.len(),
			self.uploaded.len()
		)
	}
}

/// Writes into one package's space, keeping a tally for the report.
pub(super) struct SpaceWriter {
	space: Pds,
	written: Vec<StrongRef>,
	unchanged: Vec<StrongRef>,
	uploaded: Vec<Cid>,
}

impl SpaceWriter {
	pub fn new(space: Pds) -> Self {
		Self {
			space,
			written: default(),
			unchanged: default(),
			uploaded: default(),
		}
	}

	/// The `T` at `rkey`, `None` when there is none.
	pub async fn existing<T: EntityCollection>(
		&self,
		rkey: &Rkey,
	) -> Result<Option<T>> {
		self.space
			.get::<T>(rkey)
			.await?
			.map(|entry| entry.value)
			.xok()
	}

	/// Upload `bytes`, which a record about to be written references.
	pub async fn upload(
		&mut self,
		bytes: Bytes,
		mime_type: MediaType,
	) -> Result {
		let blob = self.space.upload_blob(bytes, mime_type).await?;
		self.uploaded.push(blob.cid);
		Ok(())
	}

	/// The scene record of `document`, written with its body's blob where no
	/// record holds that content yet; one that does keeps the time it was
	/// first compiled.
	pub async fn write_scene(
		&mut self,
		document: &CompiledDocument,
	) -> Result<StrongRef> {
		let blob = document.blob_ref();
		let rkey = Rkey::parse(blob.cid.as_str())?;
		if let Some(existing) = self
			.space
			.get_record(&SceneRecord::COLLECTION, &rkey)
			.await?
		{
			self.unchanged.push(existing.strong_ref());
			return existing.strong_ref().xok();
		}
		self.upload(Bytes::from(document.blob.clone()), blob.mime_type)
			.await?;
		self.write::<SceneRecord>(document.record.clone()).await
	}

	/// Write `body` as a `T` at the rkey it derives, its [`Provenance`] the
	/// one already there where a record is, so a body whose content did not
	/// move is the same record, and left alone.
	pub async fn write<T: EntityCollection>(
		&mut self,
		mut body: EntityBodyRecord,
	) -> Result<StrongRef> {
		let rkey = T::derive_rkey(&body)?;
		let existing = self.space.get_record(&T::COLLECTION, &rkey).await?;
		let created = match &existing {
			Some(existing) => {
				EntityBodyRecord::try_from(existing.value.clone())?
					.try_get::<Provenance>()?
					.map(|provenance| provenance.created)
			}
			None => None,
		};
		body.insert(&Provenance {
			created: created.unwrap_or_else(Timestamp::now),
			..default()
		})?;
		let record = T::accept(body)?;
		let value = AtprotoValue::from(record.body().clone());
		let precondition = match existing {
			Some(existing) if existing.cid == value.cid() => {
				self.unchanged.push(existing.strong_ref());
				return existing.strong_ref().xok();
			}
			Some(existing) => Precondition::Matches(existing.cid),
			None => Precondition::Absent,
		};
		// conditional on what was read, so a concurrent compile is refused
		// rather than overwritten
		let written = self
			.space
			.put_record_if(
				&T::COLLECTION,
				Rkeyed::new(rkey, value),
				precondition,
			)
			.await?;
		self.written.push(written.clone());
		written.xok()
	}

	/// The report of everything written.
	pub fn report(
		self,
		package: Package,
		branch: BranchName,
		branch_record: StrongRef,
		package_record: StrongRef,
		dependencies: Vec<CompileReport>,
	) -> CompileReport {
		CompileReport {
			package,
			branch,
			branch_record,
			package_record,
			written: self.written,
			unchanged: self.unchanged,
			uploaded: self.uploaded,
			dependencies,
		}
	}
}
