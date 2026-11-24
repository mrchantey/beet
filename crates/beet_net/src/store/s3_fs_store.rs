use crate::prelude::*;
use beet_core::prelude::*;

/// One store over two ends, a local directory and an S3 bucket: what a
/// directory sync declares, its ends read by the sync itself
/// ([`fs_store`](Self::fs_store), [`s3_store`](Self::s3_store)).
///
/// A selector resolved once, when it is inserted: the erased [`BlobStore`]
/// and [`TableStore`] land beside it over the end its [`ServiceAccess`]
/// picks, the local one by default, so the selected store answers every
/// method itself and nothing forwards one method at a time.
#[derive(Debug, Clone, Get, SetWith, Component)]
#[component(on_insert = S3FsStore::on_insert)]
pub struct S3FsStore {
	/// Local filesystem store.
	#[set_with(skip)]
	fs_store: FsStore,
	/// Remote S3 store.
	#[set_with(skip)]
	s3_store: S3Store,
	/// Which end the erased handles are, decided at insertion.
	service_access: ServiceAccess,
}

impl S3FsStore {
	/// Create a new dual-mode store from filesystem and S3 components.
	pub fn new(fs_store: FsStore, s3_store: S3Store) -> Self {
		Self {
			fs_store,
			s3_store,
			service_access: ServiceAccess::Local,
		}
	}

	/// The end `service_access` picks, erased.
	pub fn active(&self) -> BlobStore {
		match self.service_access {
			ServiceAccess::Local => BlobStore::new(self.fs_store.clone()),
			ServiceAccess::Remote => BlobStore::new(self.s3_store.clone()),
		}
	}

	/// Land the erased handles of the active end.
	fn on_insert(mut world: DeferredWorld, cx: HookContext) {
		if let Some(store) = world.entity(cx.entity).get::<Self>() {
			let blobs = store.active();
			world
				.commands()
				.entity(cx.entity)
				.insert((TableStore::new(blobs.clone()), blobs));
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The erased handles are the selected end itself, its conditional writes
	/// included, which a forwarding selector once missed.
	#[beet_core::test]
	async fn resolves_to_the_selected_end() {
		let dir = TempDir::new_ws().unwrap();
		let mut world = World::new();
		let entity = world
			.spawn(S3FsStore::new(
				FsStore::new(dir.path().clone()),
				S3Store::new("bucket", "us-west-2"),
			))
			.id();
		world.flush();
		let blobs = world.entity(entity).get::<BlobStore>().unwrap().clone();
		blobs.id().xpect_eq("fs");
		blobs
			.insert_if(&RelPath::new("a"), "a".into(), BlobCondition::Absent)
			.await
			.unwrap();
		world
			.entity(entity)
			.get::<TableStore>()
			.unwrap()
			.table::<table_test::NamedRow>()
			.push(table_test::NamedRow::new("k", 1))
			.await
			.unwrap();
		let remote = S3FsStore::new(
			FsStore::new(dir.path().clone()),
			S3Store::new("bucket", "us-west-2"),
		)
		.with_service_access(ServiceAccess::Remote);
		remote.active().id().xpect_eq("s3");
	}
}
