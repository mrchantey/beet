use forky_web::HtmlEventListener;
use forky_web::HtmlEventWaiter;
use js_sys::Uint8Array;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;
use web_sys::Cache;
use web_sys::Event;
use web_sys::IdbDatabase;
use web_sys::Request;
use web_sys::RequestInit;
use web_sys::Response;


//https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria#how_much_data_can_be_stored

const CACHE_NAME: &str = "beet";

pub fn open_or_fetch_blocking(url: &str) {
	let url = url.to_string();
	let _ = wasm_bindgen_futures::future_to_promise(async move {
		let a = open_or_fetch(&url).await;
		log::info!("result: {:?}", a);
		Ok(JsValue::TRUE)
	});
}

pub async fn open_or_fetch(url: &str) -> Result<Uint8Array, JsValue> {
	match open_or_fetch_idb(url).await {
		Ok(data) => Ok(data),
		Err(err) => {
			log::error!(
				"failed to use indexeddb, falling back to cache: {}",
				err.as_string().unwrap_or_default()
			);
			open_or_fetch_cache(url).await
		}
	}
}

/// Attempt to retreive a model from the cache, if it doesn't exist, fetch it from the network
pub async fn open_or_fetch_cache(url: &str) -> Result<Uint8Array, JsValue> {
	let request = Request::new_with_str(url)?;

	let cache_promise = web_sys::window().unwrap().caches()?.open(CACHE_NAME);
	let cache = JsFuture::from(cache_promise).await?.dyn_into::<Cache>()?;

	let cached_response_promise = cache.match_all_with_request(&request);
	if let Ok(cached_response) = JsFuture::from(cached_response_promise)
		.await?
		.dyn_into::<Response>()
	{
		log::info!("retreived file from cache: \n{}", url);
		let data = JsFuture::from(cached_response.array_buffer()?).await?;
		return Ok(Uint8Array::new(&data));
	}

	let mut opts = RequestInit::new();
	opts.cache(web_sys::RequestCache::ForceCache);

	let res_promise = web_sys::window()
		.unwrap()
		.fetch_with_request_and_init(&request, &opts);
	let res = JsFuture::from(res_promise).await?.dyn_into::<Response>()?;

	JsFuture::from(cache.put_with_request(&request, &res.clone()?)).await?;

	let buffer = JsFuture::from(res.array_buffer()?).await?;
	log::info!("retreived file from network: \n{}", url);
	Ok(Uint8Array::new(&buffer))
}


/// Attempt to retreive a model from indexeddb, if it doesn't exist, fetch it from the network
pub async fn open_or_fetch_idb(url: &str) -> Result<Uint8Array, JsValue> {
	let idb = window()
		.unwrap()
		.indexed_db()?
		.ok_or_else(|| JsValue::from_str("no indexeddb"))?;

	const STORE_NAME: &str = "files";
	// const KEY_NAME: &str = "filename";

	let req = idb.open(CACHE_NAME)?;
	let req2 = req.clone();

	let _upgrade_listener = HtmlEventListener::new_with_target(
		"upgradeneeded",
		move |_: Event| {
			let db = req2.result().unwrap().unchecked_into::<IdbDatabase>();
			// let mut params = IdbObjectStoreParameters::new();
			// params.key_path(Some(&JsValue::from_str(KEY_NAME)));
			// db.create_object_store_with_optional_parameters(
			// 	STORE_NAME, &params,
			// )
			// .unwrap();
			db.create_object_store(STORE_NAME).unwrap();
			log::info!("created idb object store");
		},
		req.clone(),
	);

	HtmlEventWaiter::new_with_target("success", req.clone())
		.wait_or_timeout(Duration::from_millis(500))
		.await?;
	let db = req.result()?.unchecked_into::<IdbDatabase>();


	let value = idb_get(&db, STORE_NAME, url).await?;

	if value.is_undefined() {
		let request = Request::new_with_str(url)?;
		let res_promise = window().unwrap().fetch_with_request(&request);
		let res = JsFuture::from(res_promise).await?.dyn_into::<Response>()?;

		let buffer = JsFuture::from(res.array_buffer()?).await?;
		log::info!("retreived file from network");
		let value = Uint8Array::new(&buffer);

		idb_put(&db, STORE_NAME, url, &value).await?;

		Ok(value)
	} else {
		let value = value.dyn_into::<Uint8Array>()?;
		log::info!("retreived file from idb");
		Ok(value)
	}
}

async fn idb_get(
	db: &IdbDatabase,
	store_name: &str,
	key: &str,
) -> Result<JsValue, JsValue> {
	let transaction = db.transaction_with_str_and_mode(
		store_name,
		web_sys::IdbTransactionMode::Readwrite,
	)?;
	let store = transaction.object_store(store_name)?;
	let req = store.get(&JsValue::from_str(key))?;

	HtmlEventWaiter::new_with_target("success", req.clone())
		.wait_or_timeout(Duration::from_millis(500))
		.await?;

	req.result()
}
async fn idb_put(
	db: &IdbDatabase,
	store_name: &str,
	key: &str,
	value: &JsValue,
) -> Result<JsValue, JsValue> {
	let transaction = db.transaction_with_str_and_mode(
		store_name,
		web_sys::IdbTransactionMode::Readwrite,
	)?;
	let store = transaction.object_store(store_name)?;
	let req = store.put_with_key(value, &JsValue::from_str(key))?;

	HtmlEventWaiter::new_with_target("success", req.clone())
		.wait_or_timeout(Duration::from_millis(500))
		.await?;

	req.result()
}
