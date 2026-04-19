use crate::prelude::*;
use beet_core::prelude::*;

/// Parameters for listing blobs in a bucket subdirectory.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct ListBlobsParams {
	/// Subdirectory path to list relative to the bucket root.
	pub path: RelPath,
}

/// List all blobs in the given subdirectory of the nearest ancestor [`Bucket`].
///
/// Outputs a [`Vec<RelPath>`] of blob paths relative to the given subdirectory.
#[action]
#[derive(Component, Reflect)]
pub async fn ListBlobs(cx: ActionContext<ListBlobsParams>) -> Result<Vec<RelPath>> {
	let bucket = cx
		.caller
		.with_state::<AncestorQuery<&Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;
	bucket.with_subdir(cx.input.path).list().await
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn lists_empty_subdir() {
		let bucket = Bucket::temp();
		let result = bucket
			.with_subdir(RelPath::from("empty"))
			.list()
			.await
			.unwrap();
		result.xpect_eq(vec![]);
	}

	#[beet_core::test]
	async fn lists_blobs_in_subdir() {
		let bucket = Bucket::temp();
		bucket
			.insert(&RelPath::from("subdir/a.txt"), "aaa")
			.await
			.unwrap();
		bucket
			.insert(&RelPath::from("subdir/b.txt"), "bbb")
			.await
			.unwrap();
		bucket
			.insert(&RelPath::from("other/c.txt"), "ccc")
			.await
			.unwrap();

		let mut result = bucket
			.with_subdir(RelPath::from("subdir"))
			.list()
			.await
			.unwrap();
		result.sort();
		result.xpect_eq(vec![
			RelPath::from("a.txt"),
			RelPath::from("b.txt"),
		]);
	}
}
