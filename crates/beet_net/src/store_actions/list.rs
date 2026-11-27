use crate::prelude::*;
use beet_core::prelude::*;

/// The props for listing blobs in a store subdirectory.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct ListBlobsProps {
	/// Subdirectory path to list relative to the store root.
	pub path: FwdPath,
}

/// List all blobs in the given subdirectory of the nearest ancestor [`BlobStore`].
///
/// Outputs a [`Vec<FwdPath>`] of blob paths relative to the given subdirectory.
#[action]
#[derive(Component, Reflect)]
pub async fn ListBlobs(
	cx: ActionContext<ListBlobsProps>,
) -> Result<Vec<FwdPath>> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await??;
	let sub = store.with_subdir(cx.input.path);
	// gracefully return empty list if store directory doesn't exist yet
	match sub.store_exists().await {
		Ok(true) => sub.list().await,
		_ => Ok(vec![]),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn lists_empty_subdir() {
		let store = BlobStore::temp();
		let result = store
			.with_subdir(fwd_path!("empty"))
			.list()
			.await
			.unwrap();
		result.xpect_eq(vec![]);
	}

	#[beet_core::test]
	async fn lists_blobs_in_subdir() {
		let store = BlobStore::temp();
		store
			.insert(&fwd_path!("subdir/a.txt"), "aaa")
			.await
			.unwrap();
		store
			.insert(&fwd_path!("subdir/b.txt"), "bbb")
			.await
			.unwrap();
		store
			.insert(&fwd_path!("other/c.txt"), "ccc")
			.await
			.unwrap();

		let mut result = store
			.with_subdir(fwd_path!("subdir"))
			.list()
			.await
			.unwrap();
		result.sort();
		result.xpect_eq(vec![fwd_path!("a.txt"), fwd_path!("b.txt")]);
	}
}
