use crate::prelude::*;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;

/// Add this handler alongside a [`Bucket`] resource to serve files from the bucket.
/// Serves static files from the provided bucket
/// 1. If the requested path has an extension, create a permanent redirect to the public URL
/// 2. If the requested path does not have an extension, append `/index.html` and serve the file as HTML.
pub fn bucket_file_handler() -> impl Bundle {
	(RouteHandler::layer_async(async move |mut world, entity| {
		let path = world.remove_resource::<Request>().unwrap();
		let path: RoutePath = path.into();
		let entity = world.entity(entity);
		let bucket = entity.get::<Bucket>().unwrap();
		let response = from_bucket(bucket, &path).await.into_response();
		world.insert_resource(response);
		world
	}),)
}

// TODO precompressed variants, ie `index.html.br`
async fn from_bucket(bucket: &Bucket, path: &RoutePath) -> Result<Response> {
	debug!("serving from bucket: {}", path);
	if let Some(_extension) = path.extension() {
		if let Some(url) = bucket.public_url(&path).await? {
			Ok(Response::permanent_redirect(url))
		} else {
			// some buckets like fs bucket dont have a url so just serve the file directly
			bucket
				.get(path)
				.await
				.map(|bytes| Response::ok_mime_guess(bytes, path))?
				.xok()
		}
	} else {
		bucket
			.get(&path.join("index.html"))
			.await
			.map(|bytes| Response::ok_body(bytes, "text/html"))?
			.xok()
		// .map_err(|_| HttpError::not_found().into())
	}
}


#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;


	#[sweet::test]
	async fn serves_fs() {
		let bucket = Bucket::new_test().await;
		let body = "body { color: red; }";
		let path = RoutePath::from("/style.css");
		bucket.insert(&path, body).await.unwrap();
		let response = super::from_bucket(&bucket, &path).await.unwrap();
		response
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_eq(body);
	}
	#[sweet::test]
	#[ignore = "no longer redirects"]
	async fn redirect() {
		let bucket = Bucket::new_test().await;
		let path = RoutePath::from("/style.css");
		bucket.insert(&path, "body { color: red; }").await.unwrap();
		let path = RoutePath::from("/style.css");
		let response = super::from_bucket(&bucket, &path).await.unwrap();
		response.status().xpect_eq(StatusCode::MOVED_PERMANENTLY);
		response
			.header(header::LOCATION)
			.unwrap()
			.unwrap()
			.xpect_ends_with("/style.css");
	}
	#[sweet::test]
	async fn html() {
		let bucket = Bucket::new_test().await;
		let body = "<h1>Hello, world!</h1>";
		bucket
			.insert(&RoutePath::from("/docs/index.html"), body)
			.await
			.unwrap();
		let response = super::from_bucket(&bucket, &RoutePath::from("/docs"))
			.await
			.unwrap();
		response
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_eq(body);
	}
	#[sweet::test]
	async fn as_fallback() {
		let bucket = Bucket::new_test().await;
		let path = RoutePath::from("/index.html");
		bucket.insert(&path, "<div>fallback</div>").await.unwrap();
		Router::new_bundle(move || {
			(
				HandlerConditions::fallback(),
				bucket.clone(),
				super::bucket_file_handler(),
			)
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect_str("<div>fallback</div>");
	}
}
