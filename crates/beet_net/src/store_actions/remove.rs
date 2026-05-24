use crate::prelude::*;
use beet_core::prelude::*;

/// Parameters for removing a blob from a store.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct RemoveBlobParams {
	/// Path to the blob to remove.
	pub path: RelPath,
}

/// Remove a blob from the nearest ancestor [`BlobStore`].
#[action]
#[derive(Component, Reflect)]
pub async fn RemoveBlob(cx: ActionContext<RemoveBlobParams>) -> Result<()> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await??;
	store.remove(&cx.input.path).await
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn removes_existing_blob() {
		let store = BlobStore::temp();
		let path = RelPath::from("file.txt");
		store.insert(&path, "hello").await.unwrap();
		store.exists(&path).await.unwrap().xpect_true();
		store.remove(&path).await.unwrap();
		store.exists(&path).await.unwrap().xpect_false();
	}
}
