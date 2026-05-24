//! Cross-platform model fetch with on-device caching.
//!
//! Web targets cache in IndexedDB; native targets cache to disk under
//! the OS temp directory. The single public entry point is [`fetch_bytes`].

use beet_core::prelude::*;
use beet_net::prelude::*;

/// Download `url` (or load the cached copy) and return the bytes.
///
/// Cache lookups happen first; on a miss the bytes are downloaded over
/// HTTP and written back to the cache before returning.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
	let store = cache_store();
	let key = cache_key(url);

	if store.exists(&key).await.unwrap_or(false) {
		log::info!("fetch_bytes: cache hit ({url})");
		return store.get(&key).await.map(|b| b.to_vec());
	}

	log::info!("fetch_bytes: cache miss, downloading ({url})");
	let bytes = Request::get(url)
		.send()
		.await?
		.into_result()
		.await?
		.bytes_vec()
		.await?;

	if let Err(err) = store.insert(&key, bytes.clone()).await {
		log::warn!("fetch_bytes: cache write failed: {err}");
	}
	Ok(bytes)
}

const STORE_NAME: &str = "beet_ml_cache";

fn cache_store() -> BlobStore {
	cfg_if! {
		if #[cfg(not(target_arch = "wasm32"))] {
			BlobStore::new(FsStore::new(AbsPathBuf::new_workspace_rel(format!("target/{STORE_NAME}")).unwrap()))
		} else if #[cfg(target_arch = "wasm32")] {
			BlobStore::new(IndexedDbStore::new(STORE_NAME))
		}
	}
}

/// Hash the URL to keep cache keys short and filesystem-safe.
fn cache_key(url: &str) -> RelPath {
	RelPath::new(format!("{:016x}.bin", fs_ext::hash_string(url)))
}
