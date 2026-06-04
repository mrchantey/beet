use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::IdbDatabase;
use web_sys::IdbTransactionMode;

/// A store provider backed by browser IndexedDB.
///
/// Suited to large binary payloads (ie ML model weights) that exceed the
/// per-origin `localStorage` quota. Each store uses a dedicated database
/// with a single object store keyed by [`SmolPath`] strings.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = BlobStore::on_add::<Self>)]
pub struct IndexedDbStore {
	/// The IndexedDB database name.
	db_name: SmolStr,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<SmolPath>,
}

const STORE_NAME: &str = "blobs";

impl IndexedDbStore {
	/// Creates a new IndexedDB-backed store provider.
	pub fn new(db_name: impl Into<SmolStr>) -> Self {
		Self {
			db_name: db_name.into(),
			subdir: None,
		}
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<SmolPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	fn effective_key(&self, path: &SmolPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		}
	}
}

async fn open_db(db_name: &str) -> Result<IdbDatabase> {
	let factory = web_sys::window()
		.ok_or_else(|| bevyhow!("idb: no window"))?
		.indexed_db()
		.map_jserr()?
		.ok_or_else(|| bevyhow!("idb: not supported"))?;

	let req = factory.open(db_name).map_jserr()?;

	// create the object store on first open
	let onupgrade = Closure::<dyn FnMut(web_sys::Event)>::new({
		let req = req.clone();
		move |_| {
			if let Ok(db) = req.result() {
				let db: IdbDatabase = db.unchecked_into();
				// `create_object_store` errors if it already exists, which we
				// can ignore — this hook only fires on schema upgrade.
				let _ = db.create_object_store(STORE_NAME);
			}
		}
	});
	req.set_onupgradeneeded(Some(onupgrade.as_ref().unchecked_ref()));
	onupgrade.forget();

	let (sender, receiver) = futures::channel::oneshot::channel();
	let sender = std::cell::RefCell::new(Some(sender));
	let onsuccess = Closure::<dyn FnMut(web_sys::Event)>::new({
		let req = req.clone();
		move |_| {
			if let Some(tx) = sender.borrow_mut().take() {
				let _ = tx.send(req.result());
			}
		}
	});
	req.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
	onsuccess.forget();

	receiver
		.await
		.map_err(|e| bevyhow!("idb open cancelled: {e}"))?
		.map_jserr()?
		.dyn_into::<IdbDatabase>()
		.map_err(|e| bevyhow!("idb db cast: {e:?}"))
}

async fn await_idb_request(req: web_sys::IdbRequest) -> Result<JsValue> {
	let (sender, receiver) = futures::channel::oneshot::channel();
	let sender = std::cell::RefCell::new(Some(sender));
	let req_clone = req.clone();
	let onsuccess = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
		if let Some(tx) = sender.borrow_mut().take() {
			let _ = tx.send(req_clone.result());
		}
	});
	req.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
	onsuccess.forget();
	receiver
		.await
		.map_err(|e| bevyhow!("idb request cancelled: {e}"))?
		.map_jserr()
}

impl BlobStoreProvider for IndexedDbStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		Box::new(IndexedDbStore {
			db_name: self.db_name.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn id(&self) -> &'static str { "indexeddb" }

	fn root_key(&self) -> SmolStr {
		format!("indexeddb:{}", self.db_name).into()
	}

	fn subdir(&self) -> SmolPath { self.subdir.clone().unwrap_or_default() }

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let db_name = self.db_name.clone();
		Box::pin(SendWrapper::new(async move {
			open_db(&db_name).await.map(|_| true)
		}))
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let db_name = self.db_name.clone();
		Box::pin(SendWrapper::new(async move {
			open_db(&db_name).await?;
			Ok(())
		}))
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let db_name = self.db_name.clone();
		Box::pin(SendWrapper::new(async move {
			let factory = web_sys::window()
				.ok_or_else(|| bevyhow!("idb: no window"))?
				.indexed_db()
				.map_jserr()?
				.ok_or_else(|| bevyhow!("idb: not supported"))?;
			let req = factory.delete_database(&db_name).map_jserr()?;
			let (sender, receiver) = futures::channel::oneshot::channel();
			let sender = std::cell::RefCell::new(Some(sender));
			let onsuccess =
				Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
					if let Some(tx) = sender.borrow_mut().take() {
						let _ = tx.send(());
					}
				});
			req.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
			onsuccess.forget();
			receiver
				.await
				.map_err(|e| bevyhow!("idb delete cancelled: {e}"))?;
			Ok(())
		}))
	}

	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		let db_name = self.db_name.clone();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			let db = open_db(&db_name).await?;
			let tx = db
				.transaction_with_str_and_mode(
					STORE_NAME,
					IdbTransactionMode::Readwrite,
				)
				.map_jserr()?;
			let store = tx.object_store(STORE_NAME).map_jserr()?;
			let arr = Uint8Array::new_with_length(body.len() as u32);
			arr.copy_from(&body);
			let req = store
				.put_with_key(&arr, &JsValue::from_str(&key))
				.map_jserr()?;
			await_idb_request(req).await?;
			Ok(())
		}))
	}

	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		let db_name = self.db_name.clone();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			let db = open_db(&db_name).await?;
			let tx = db
				.transaction_with_str_and_mode(
					STORE_NAME,
					IdbTransactionMode::Readonly,
				)
				.map_jserr()?;
			let store = tx.object_store(STORE_NAME).map_jserr()?;
			let req = store.get(&JsValue::from_str(&key)).map_jserr()?;
			let value = await_idb_request(req).await?;
			(!value.is_undefined() && !value.is_null()).xok()
		}))
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		let db_name = self.db_name.clone();
		let subdir_prefix = self.subdir.as_ref().map(|s| format!("{}/", s));
		Box::pin(SendWrapper::new(async move {
			let db = open_db(&db_name).await?;
			let tx = db
				.transaction_with_str_and_mode(
					STORE_NAME,
					IdbTransactionMode::Readonly,
				)
				.map_jserr()?;
			let store = tx.object_store(STORE_NAME).map_jserr()?;
			let req = store.get_all_keys().map_jserr()?;
			let keys_val = await_idb_request(req).await?;
			let keys: js_sys::Array = keys_val.unchecked_into();
			let paths = (0..keys.length())
				.filter_map(|i| keys.get(i).as_string())
				.filter_map(|key| match &subdir_prefix {
					Some(p) => key
						.strip_prefix(p.as_str())
						.map(|rest| SmolPath::new(rest)),
					None => Some(SmolPath::new(&key)),
				})
				.collect::<Vec<_>>();
			paths.xok()
		}))
	}

	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		let db_name = self.db_name.clone();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			let db = open_db(&db_name).await?;
			let tx = db
				.transaction_with_str_and_mode(
					STORE_NAME,
					IdbTransactionMode::Readonly,
				)
				.map_jserr()?;
			let store = tx.object_store(STORE_NAME).map_jserr()?;
			let req = store.get(&JsValue::from_str(&key)).map_jserr()?;
			let value = await_idb_request(req).await?;
			if value.is_undefined() || value.is_null() {
				bevybail!("Object not found: {}", key);
			}
			Bytes::from(Uint8Array::new(&value).to_vec()).xok()
		}))
	}

	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let db_name = self.db_name.clone();
		let key = self.effective_key(path);
		let path = path.clone();
		Box::pin(SendWrapper::new(async move {
			// Resolve exists synchronously by re-opening; the trait's
			// `exists` future can't be awaited inside a SendWrapper from
			// here without giving up Send-ness on the wrapper.
			let _ = this;
			let db = open_db(&db_name).await?;
			let tx = db
				.transaction_with_str_and_mode(
					STORE_NAME,
					IdbTransactionMode::Readonly,
				)
				.map_jserr()?;
			let store = tx.object_store(STORE_NAME).map_jserr()?;
			let req = store.get(&JsValue::from_str(&key)).map_jserr()?;
			let value = await_idb_request(req).await?;
			if value.is_undefined() || value.is_null() {
				bevybail!("Object not found: {}", path);
			}
			let tx = db
				.transaction_with_str_and_mode(
					STORE_NAME,
					IdbTransactionMode::Readwrite,
				)
				.map_jserr()?;
			let store = tx.object_store(STORE_NAME).map_jserr()?;
			let req = store.delete(&JsValue::from_str(&key)).map_jserr()?;
			await_idb_request(req).await?;
			Ok(())
		}))
	}

	fn public_url(
		&self,
		_path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(SendWrapper::new(async move { None.xok() }))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	// IndexedDB requires a browser environment; the beet wasm test runner
	// uses Deno which has no `window.indexedDB`.
	#[beet_core::test]
	#[ignore = "requires browser environment"]
	async fn works() {
		let provider = IndexedDbStore::new("test-store");
		store_test::run(provider).await;
	}
}
