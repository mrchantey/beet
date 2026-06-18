use crate::prelude::*;
use beet_core::prelude::*;

/// Parameters for writing a blob to a store.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct WriteBlobParams {
	/// Path to write the blob to.
	pub path: SmolPath,
	/// Raw bytes to write.
	pub bytes: Vec<u8>,
}

/// Completely replace a blob in the nearest ancestor [`BlobStore`].
///
/// Emits a [`BlobEvent`] on success, covering the wasm same-tab gap and giving
/// s3 reactivity that no external watcher provides.
#[action]
#[derive(Component, Reflect)]
pub async fn WriteBlob(cx: ActionContext<WriteBlobParams>) -> Result<()> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await??;
	// existence picks Created vs Changed so a new object refreshes the listing
	let existed = store.exists(&cx.input.path).await.unwrap_or(false);
	store.insert(&cx.input.path, cx.input.bytes).await?;
	let kind = match existed {
		true => BlobEventKind::Changed,
		false => BlobEventKind::Created,
	};
	cx.caller
		.world()
		.trigger(BlobEvent::new(store, cx.input.path, kind))
		.await;
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn writes_and_reads_back() {
		let store = BlobStore::temp();
		let path = SmolPath::from("file.bin");
		let data: Vec<u8> = vec![1, 2, 3, 4, 5];
		store.insert(&path, data.clone()).await.unwrap();
		let got = store.get(&path).await.unwrap();
		got.to_vec().xpect_eq(data);
	}

	#[beet_core::test]
	async fn overwrites_existing() {
		let store = BlobStore::temp();
		let path = SmolPath::from("file.txt");
		store.insert(&path, "first").await.unwrap();
		store.insert(&path, "second").await.unwrap();
		let got = store.get(&path).await.unwrap();
		got.to_vec().xpect_eq(b"second".to_vec());
	}
}
