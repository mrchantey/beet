use crate::prelude::*;
use beet_core::prelude::*;

/// The props for writing bytes to a store.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct WriteBlobProps {
	/// Path to write the blob to.
	pub path: RelPath,
	/// Raw bytes to write.
	pub bytes: Vec<u8>,
}

/// Completely replace a blob in the nearest ancestor [`BlobStore`] with bytes.
///
/// Emits a [`BlobEvent`] on success, covering the wasm same-tab gap and giving
/// s3 reactivity that no external watcher provides. An agent writes through
/// [`WriteText`] instead, since a model spells a byte array badly.
#[action]
#[derive(Component, Reflect)]
pub async fn WriteBlob(cx: ActionContext<WriteBlobProps>) -> Result<()> {
	let WriteBlobProps { path, bytes } = cx.input;
	WriteBlob::write(&cx.caller, path, bytes).await
}

/// The props for writing text to a store.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct WriteTextProps {
	/// Path to write the text to.
	pub path: RelPath,
	/// The blob's whole text, written as UTF-8.
	pub text: String,
}

/// Completely replace a blob in the nearest ancestor [`BlobStore`] with text,
/// the agent tool `write-text`: what an agent writes is a document, a spec or
/// a note, and a model spells text far more reliably than a byte array.
///
/// Emits a [`BlobEvent`] on success, as [`WriteBlob`] does.
#[action]
#[derive(Component, Reflect)]
pub async fn WriteText(cx: ActionContext<WriteTextProps>) -> Result<()> {
	let WriteTextProps { path, text } = cx.input;
	WriteBlob::write(&cx.caller, path, text.into_bytes()).await
}

impl WriteBlob {
	/// Replace the blob at `path` in the store at or above `caller` with
	/// `bytes`, emitting the [`BlobEvent`] it caused.
	async fn write(
		caller: &AsyncEntity,
		path: RelPath,
		bytes: Vec<u8>,
	) -> Result {
		let store = caller
			.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
				query.get(entity).cloned()
			})
			.await??;
		// existence picks Created vs Changed so a new object refreshes the listing
		let existed = store.exists(&path).await.unwrap_or(false);
		store.insert(&path, bytes).await?;
		let kind = match existed {
			true => BlobEventKind::Changed,
			false => BlobEventKind::Created,
		};
		caller
			.world()
			.trigger(BlobEvent::new(store, path, kind))
			.await;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	/// Call the write `action` with `input` over `store`, answering the blob
	/// at `file.bin`.
	async fn write<In: 'static + Send + Sync>(
		store: &BlobStore,
		action: impl Bundle,
		input: In,
	) -> Vec<u8> {
		AsyncPlugin::world()
			.spawn((store.clone(), action))
			.call::<In, ()>(input)
			.await
			.unwrap();
		store
			.get(&RelPath::from("file.bin"))
			.await
			.unwrap()
			.to_vec()
	}

	#[beet_core::test]
	async fn writes_bytes() {
		let bytes = vec![0, 159, 146, 150];
		write(&BlobStore::temp(), WriteBlob, WriteBlobProps {
			path: RelPath::from("file.bin"),
			bytes: bytes.clone(),
		})
		.await
		.xpect_eq(bytes);
	}

	#[beet_core::test]
	async fn writes_text_over_bytes() {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::from("file.bin"), vec![1, 2])
			.await
			.unwrap();
		write(&store, WriteText, WriteTextProps {
			path: RelPath::from("file.bin"),
			text: "hello".into(),
		})
		.await
		.xpect_eq(b"hello".to_vec());
	}
}
