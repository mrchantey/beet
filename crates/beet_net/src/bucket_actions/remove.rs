use crate::prelude::*;
use beet_core::prelude::*;

/// Parameters for removing a blob from a bucket.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct RemoveBlobParams {
	/// Path to the blob to remove.
	pub path: RelPath,
}

/// Remove a blob from the nearest ancestor [`Bucket`].
#[action]
#[derive(Component, Reflect)]
pub async fn RemoveBlob(cx: ActionContext<RemoveBlobParams>) -> Result<()> {
	let bucket = cx
		.caller
		.with_state::<AncestorQuery<&Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;
	bucket.remove(&cx.input.path).await
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn removes_existing_blob() {
		let bucket = Bucket::temp();
		let path = RelPath::from("file.txt");
		bucket.insert(&path, "hello").await.unwrap();
		bucket.exists(&path).await.unwrap().xpect_true();
		bucket.remove(&path).await.unwrap();
		bucket.exists(&path).await.unwrap().xpect_false();
	}
}
