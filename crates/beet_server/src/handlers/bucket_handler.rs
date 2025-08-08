use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;




/// Add this handler alongside a [`Bucket`] resource to serve files from the bucket.
/// Serves static files from the provided bucket
/// 1. If the requested path has an extension, create a permanent redirect to the public URL
/// 2. If the requested path does not have an extension, append `/index.html` and serve the file as HTML.
pub fn bucket_handler() -> impl Bundle {
	RouteHandler::layer_async(async move |mut world, entity| {
		let path = world.remove_resource::<Request>().unwrap();
		let path: RoutePath = path.into();
		let entity = world.entity(entity);
		let bucket = entity.get::<Bucket>().unwrap();
		let response = from_bucket(bucket, path).await.into_response();
		world.insert_resource(response);
		world
	})
}

// TODO precompressed variants, ie `index.html.br`
async fn from_bucket(bucket: &Bucket, path: RoutePath) -> Result<Response> {
	debug!("serving from bucket: {}", path);
	if let Some(_extension) = path.extension() {
		let url = bucket.public_url(&path.to_string()).await?;
		if url.starts_with("file://") {
			// the fs bucket will return a file:// url which is an ERR_UNSAFE_REDIRECT
			// so just serve the file directly
			bucket
				.get(&path.to_string())
				.await
				.map(|bytes| Response::ok_mime_guess(bytes, path))
				.map_err(|_| HttpError::not_found().into())
		} else {
			Ok(Response::permanent_redirect(url))
		}
	} else {
		let path = path.join("index.html");
		bucket
			.get(&path.to_string())
			.await
			.map(|bytes| Response::ok_body(bytes, "text/html"))
			.map_err(|_| HttpError::not_found().into())
	}
}


#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use http::header::LOCATION;
	use sweet::prelude::*;


	#[sweet::test]
	async fn serves_fs() {
		let (bucket, _drop) = Bucket::new_test().await;
		let body = "body { color: red; }";
		bucket.insert("style.css", body).await.unwrap();
		let path = RoutePath::from("/style.css");
		let response = super::from_bucket(&bucket, path).await.unwrap();
		response
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect()
			.to_be(body);
	}
	#[sweet::test]
	#[ignore = "no longer redirects"]
	async fn redirect() {
		let (bucket, _drop) = Bucket::new_test().await;
		bucket
			.insert("style.css", "body { color: red; }")
			.await
			.unwrap();
		let path = RoutePath::from("/style.css");
		let response = super::from_bucket(&bucket, path).await.unwrap();
		response
			.status()
			.xpect()
			.to_be(StatusCode::MOVED_PERMANENTLY);
		response
			.header(LOCATION)
			.unwrap()
			.unwrap()
			.xpect()
			.to_end_with("/style.css");
	}
	#[sweet::test]
	async fn html() {
		let (bucket, _drop) = Bucket::new_test().await;
		let path = "docs/index.html";
		let body = "<h1>Hello, world!</h1>";
		bucket.insert(path, body).await.unwrap();
		let path = RoutePath::from("docs");
		let response = super::from_bucket(&bucket, path).await.unwrap();
		response
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect()
			.to_be(body);
	}
	#[sweet::test]
	async fn as_fallback() {
		let (bucket, _drop) = Bucket::new_test().await;
		bucket
			.insert("index.html", "<div>fallback</div>")
			.await
			.unwrap();
		Router::new(move |app: &mut App| {
			app.world_mut().spawn((
				HandlerConditions::fallback(),
				bucket.clone(),
				super::bucket_handler(),
			));
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("<div>fallback</div>");
	}
}
