use crate::prelude::*;
use beet_core::prelude::*;

/// Parameters for writing a blob to a bucket.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct WriteBlobParams {
	/// Path to write the blob to.
	pub path: RelPath,
	/// Raw bytes to write.
	pub bytes: Vec<u8>,
}

/// Completely replace a blob in the nearest ancestor [`Bucket`].
#[action]
#[derive(Component, Reflect)]
pub async fn WriteBlob(cx: ActionContext<WriteBlobParams>) -> Result<()> {
	let bucket = cx
		.caller
		.with_state::<AncestorQuery<&Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;
	bucket.insert(&cx.input.path, cx.input.bytes).await
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn writes_and_reads_back() {
		let bucket = Bucket::temp();
		let path = RelPath::from("file.bin");
		let data: Vec<u8> = vec![1, 2, 3, 4, 5];
		bucket.insert(&path, data.clone()).await.unwrap();
		let got = bucket.get(&path).await.unwrap();
		got.to_vec().xpect_eq(data);
	}

	#[beet_core::test]
	async fn overwrites_existing() {
		let bucket = Bucket::temp();
		let path = RelPath::from("file.txt");
		bucket.insert(&path, "first").await.unwrap();
		bucket.insert(&path, "second").await.unwrap();
		let got = bucket.get(&path).await.unwrap();
		got.to_vec().xpect_eq(b"second".to_vec());
	}
}
