//! Cross-platform model fetch with on-device caching.
//!
//! Browsers cache in IndexedDB; every host with a filesystem caches under
//! `target/stores`. The single public entry point is [`fetch_bytes`].

use beet_core::prelude::*;
use beet_net::prelude::*;

/// Download `url` (or load the cached copy) and return the bytes.
///
/// Cache lookups happen first; on a miss the bytes are downloaded over
/// HTTP and written back to the cache before returning.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
	let store = cache_store()?;
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

/// The cache is the host's local store named [`STORE_NAME`]: a workspace
/// directory where there is a filesystem, IndexedDB in a browser.
fn cache_store() -> Result<BlobStore> {
	BlobStore::from_uri(&ServiceAccess::local_store_uri(STORE_NAME))
}

/// Hash the URL to keep cache keys short and filesystem-safe.
fn cache_key(url: &str) -> SmolPath {
	SmolPath::new(format!("{:016x}.bin", fs_ext::hash_string(url)))
}
