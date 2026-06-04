use crate::prelude::*;
use beet_core::prelude::*;

/// Parameters for listing blobs in a store subdirectory.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct ListBlobsParams {
	/// Subdirectory path to list relative to the store root.
	pub path: SmolPath,
}

/// List all blobs in the given subdirectory of the nearest ancestor [`BlobStore`].
///
/// Outputs a [`Vec<SmolPath>`] of blob paths relative to the given subdirectory.
#[action]
#[derive(Component, Reflect)]
pub async fn ListBlobs(
	cx: ActionContext<ListBlobsParams>,
) -> Result<Vec<SmolPath>> {
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
			.with_subdir(SmolPath::from("empty"))
			.list()
			.await
			.unwrap();
		result.xpect_eq(vec![]);
	}

	#[beet_core::test]
	async fn lists_blobs_in_subdir() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("subdir/a.txt"), "aaa")
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("subdir/b.txt"), "bbb")
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("other/c.txt"), "ccc")
			.await
			.unwrap();

		let mut result = store
			.with_subdir(SmolPath::from("subdir"))
			.list()
			.await
			.unwrap();
		result.sort();
		result.xpect_eq(vec![SmolPath::from("a.txt"), SmolPath::from("b.txt")]);
	}
}
