//! The record family: [`BranchRecord`], [`PackageRecord`], [`ReleaseRecord`],
//! [`SceneRecord`], [`EntityRecord`] and [`HomeRecord`], each an
//! [`EntityBodyRecord`] whose identity components derive its rkey.
use beet_core::prelude::*;

/// Declare a record type over an [`EntityBodyRecord`]: serialized as the body
/// alone, read back through [`EntityCollection::accept`].
macro_rules! entity_record {
	(
		$(#[$meta:meta])*
		$name:ident,
		collection: $collection:literal,
		required: [$($required:ty),* $(,)?],
		optional: [$($optional:ty),* $(,)?],
		rkey: $rkey:expr $(,)?
	) => {
		$(#[$meta])*
		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
		#[serde(try_from = "EntityBodyRecord", into = "EntityBodyRecord")]
		pub struct $name {
			body: EntityBodyRecord,
			rkey: Rkey,
		}

		impl AtprotoRecord for $name {
			const COLLECTION: Nsid = Nsid::new_static($collection);
		}

		impl EntityCollection for $name {
			const REQUIRED: &'static [Nsid] = &[$(<$required>::NSID),*];
			const OPTIONAL: &'static [Nsid] = &[$(<$optional>::NSID),*];
			fn derive_rkey(body: &EntityBodyRecord) -> Result<Rkey> {
				let derive: fn(&EntityBodyRecord) -> Result<Rkey> = $rkey;
				derive(body)
			}
			fn from_parts(body: EntityBodyRecord, rkey: Rkey) -> Self {
				Self { body, rkey }
			}
			fn body(&self) -> &EntityBodyRecord { &self.body }
			fn rkey(&self) -> &Rkey { &self.rkey }
		}

		impl TryFrom<EntityBodyRecord> for $name {
			type Error = BevyError;
			fn try_from(body: EntityBodyRecord) -> Result<Self> {
				Self::accept(body)
			}
		}

		impl From<$name> for EntityBodyRecord {
			fn from(record: $name) -> Self { record.body }
		}

		impl $name {
			/// The record paired with its rkey, ready to write.
			pub fn rkeyed(&self) -> Rkeyed<Self> {
				Rkeyed::new(self.rkey.clone(), self.clone())
			}

			/// The body, every component as stored.
			pub fn into_body(self) -> EntityBodyRecord { self.body }
		}
	};
}

entity_record! {
	/// `org.beet.branch`: a package's working files, in the package's space,
	/// rkey `<package>:<branch>`, ie `todos:main`. Rewritten by every edit and
	/// holding nothing compiled, so a runtime with no compiler can still
	/// write it: the branch compiled is the [`PackageRecord`] at the same
	/// rkey. Beside [`Files`] sit the entry document's declarations, so a
	/// resolver reads them without compiling.
	BranchRecord,
	collection: "org.beet.branch",
	required: [Provenance, Branch],
	optional: [Files, Package, Exports, Dependencies, ExportRuntimes],
	rkey: |body| body.get::<Branch>().map(|branch| branch.rkey()),
}

entity_record! {
	/// `org.beet.package`: one branch compiled, in the package's space, rkey
	/// `<package>:<branch>` like the branch record it was compiled from.
	/// Rewritten by every compile, and stale exactly when the branch's current
	/// cid is no longer the one [`SourceBranch`] names. What a development
	/// runtime loads, and what a release freezes, so releasing copies this
	/// body and recompiles nothing.
	PackageRecord,
	collection: "org.beet.package",
	required: [Provenance, Branch, SourceBranch],
	optional: [
		Package,
		Scenes,
		Files,
		Exports,
		Dependencies,
		ExportRuntimes,
		RequireCfg,
		LoadSets,
	],
	rkey: |body| body.get::<Branch>().map(|branch| branch.rkey()),
}

entity_record! {
	/// `org.beet.release`: a package record frozen under its version, rkey
	/// `<package>:<version>`, ie `todos:1.2.3`, `todos:1.2.3-alpha.3`. The
	/// package record's body copied whole, every component untouched, so a
	/// release and its package record differ in collection and identity
	/// alone and nothing is recompiled. Created in the package's space and
	/// never put, so a version cannot be rewritten; the newest with no
	/// prerelease is also copied whole to `<package>:latest`, the one mutable
	/// rkey. Its cid commits to the whole version: [`Scenes`] names each scene
	/// by strong ref and [`Files`] every blob. [`PackageReleaser`] writes it.
	ReleaseRecord,
	collection: "org.beet.release",
	required: [Provenance, Package, SourceBranch],
	optional: [
		Branch,
		Scenes,
		Files,
		Exports,
		Dependencies,
		ExportRuntimes,
		RequireCfg,
		LoadSets,
	],
	rkey: |body| {
		let package = body.get::<Package>()?;
		Rkey::parse(&format!("{}:{}", package.name, package.version))
	},
}

entity_record! {
	/// `org.beet.scene`: one document compiled, keyed by content, rkey the cid
	/// of its body's blob, postcard bytes of a [`SceneBody`]. Two branches or
	/// releases whose document did not change point at the same record, so
	/// nothing is duplicated and no path or version is folded into a key.
	///
	/// Pure content: where it came from is the package record's [`SceneRef`].
	/// Beside the body's [`BlobRef`] sits the scene meta, what every
	/// [`SceneMeta`] hoisted from this document alone, a plugin's under its
	/// own lexicon and inert in a runtime without the plugin.
	SceneRecord,
	collection: "org.beet.scene",
	required: [Provenance, BlobRef],
	optional: [
		Imports,
		Props,
		RequireCfg,
		Package,
		Exports,
		Dependencies,
		ExportRuntimes,
	],
	rkey: |body| {
		body.get::<BlobRef>().and_then(|blob| Rkey::parse(blob.cid.as_str()))
	},
}

entity_record! {
	/// `org.beet.entity`: an entity nobody has blessed with a collection,
	/// [`Provenance`] and whatever else was hoisted, keyed by the TID of its
	/// creation since nothing else identifies it. A derived file is one.
	EntityRecord,
	collection: "org.beet.entity",
	required: [Provenance],
	optional: [],
	rkey: |body| {
		body.get::<Provenance>()
			.map(|provenance| Rkey::from(Tid::from(provenance.created)))
	},
}

entity_record! {
	/// `org.beet.home`: the account's home package by name, one per account,
	/// rkey `self`, in the public repo. `@handle` alone resolves the handle,
	/// its did, this record, then `<package>:latest`. Written only by a
	/// publish asked to (`beet publish --as-home`), never implicitly.
	HomeRecord,
	collection: "org.beet.home",
	required: [Provenance, Home],
	optional: [],
	rkey: |_| Rkey::parse(HomeRecord::RKEY),
}

impl HomeRecord {
	/// The one rkey a home record lives at.
	pub const RKEY: &'static str = "self";
}

#[cfg(test)]
mod test {
	use super::*;

	fn branch() -> Branch {
		Branch::new(
			PackageName::parse("todos").unwrap(),
			BranchName::parse("dev").unwrap(),
		)
	}

	#[beet_core::test]
	fn derives_its_rkey() {
		let body = EntityBodyRecord::new(BranchRecord::COLLECTION)
			.with(&Provenance::default())
			.unwrap()
			.with(&branch())
			.unwrap();
		BranchRecord::accept(body)
			.unwrap()
			.rkey()
			.as_str()
			.xpect_eq("todos:dev");
	}

	#[beet_core::test]
	fn refuses_a_missing_identity_or_another_collection() {
		let body = EntityBodyRecord::new(PackageRecord::COLLECTION)
			.with(&Provenance::default())
			.unwrap()
			.with(&branch())
			.unwrap();
		PackageRecord::accept(body.clone())
			.unwrap_err()
			.to_string()
			.xpect_contains("org.beet.core#sourceBranch");
		BranchRecord::accept(body)
			.unwrap_err()
			.to_string()
			.xpect_contains("expected a `org.beet.branch` record");
	}

	/// A record round trips through the data model, a plugin's component kept.
	#[beet_core::test]
	fn round_trips_through_the_data_model() {
		let mut body = EntityBodyRecord::new(SceneRecord::COLLECTION)
			.with(&Provenance::default())
			.unwrap()
			.with(&BlobRef::of(b"body", MediaType::Postcard))
			.unwrap();
		body.insert_value(
			Nsid::new_static("com.example.plugin#route"),
			value!({ "scale": 0.5 }).try_into().unwrap(),
		);
		let record = SceneRecord::accept(body).unwrap();
		let value = AtprotoValue::from_serde(&record).unwrap();
		value.into_serde::<SceneRecord>().unwrap().xpect_eq(record);
	}
}
