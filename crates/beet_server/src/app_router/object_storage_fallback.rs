use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;




/// A marker component added to a [`Bucket`] to indicate it is used as an Object Storage Fallback.
/// There should be only one such bucket in the app.
/// The object storage fallback has two functions:
/// 1. If the requested path has an extension, create a permanent redirect to the public URL
/// 2. If the requested path does not have an extension, append `/index.html` and serve the file as HTML.
/// Serving static files as redirects lightens the load on the server, 
/// and serving html through the server makes cors simpler.
#[derive(Component)]
pub struct ObjectStorageFallback;


pub async fn object_storage_fallback(
	world: &mut World,
	path: RoutePath,
) -> Result<Response> {
	let bucket = world
		.query_filtered::<&Bucket, With<ObjectStorageFallback>>()
		.single(world)?;

	if let Some(_extension) = path.extension() {
		let url = bucket.public_url(&path.to_string()).await?;
		Ok(Response::permanent_redirect(url))
	} else {
		let path = path.join("index.html");
		match bucket.get(&path.to_string()).await {
			Ok(bytes) => Ok(Response::ok_body(bytes, "text/html")),
			Err(_) => Ok(StatusCode::NOT_FOUND.into_response()),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use http::header::LOCATION;
	use sweet::prelude::*;

	#[sweet::test]
	async fn redirect() {
		let mut world = World::new();
		let (bucket, _drop) = Bucket::new_test().await;
		world.spawn((ObjectStorageFallback, bucket));
		let path = RoutePath::from("style.css");
		let response = object_storage_fallback(&mut world, path).await.unwrap();
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
		let mut world = World::new();
		let (bucket, _drop) = Bucket::new_test().await;
		let path = "docs/index.html";
		let body = "<h1>Hello, world!</h1>";
		bucket.insert(path, body).await.unwrap();
		world.spawn((ObjectStorageFallback, bucket));
		let path = RoutePath::from("docs");
		let response = object_storage_fallback(&mut world, path).await.unwrap();
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
}
