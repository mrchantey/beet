use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


pub struct BucketEndpoint;

impl BucketEndpoint {
	/// Add this handler alongside a [`Bucket`] resource to serve files from the bucket.
	/// Serves static files from the provided bucket
	/// 1. If the requested path has an extension, create a permanent redirect to the public URL
	/// 2. If the requested path does not have an extension, append `/index.html` and serve the file as HTML.
	///
	///
	/// ## Errors
	/// If `remove_prefix` is supplied and not present in the request path.
	/// This handler **must** have a `PathFilter` matching the prefix in its ancestors.
	pub fn new(
		bucket: Bucket,
		remove_prefix: Option<RoutePath>,
	) -> impl Bundle {
		EndpointBuilder::default()
			.with_trailing_path()
			.with_handler_bundle((
				bucket,
				async move |mut path: RoutePath,
				            cx: EndpointContext|
				            -> Result<Response> {
					if let Some(prefix) = &remove_prefix {
						if let Ok(stripped) = path.strip_prefix(prefix) {
							path = RoutePath::new(stripped);
						} else {
							bevybail!("prefix {prefix} not found in {path}");
						}
					}
					let bucket = cx.action().get_cloned::<Bucket>().await?;
					bucket_to_response(&bucket, &path)
						.await?
						.into_response()
						.xok()
				}
				.into_endpoint(),
			))
	}
}



// TODO precompressed variants, ie `index.html.br`
async fn bucket_to_response(
	bucket: &Bucket,
	path: &RoutePath,
) -> Result<Response> {
	if let Some(_extension) = path.extension() {
		if let Some(url) = bucket.public_url(&path).await? {
			debug!("redirecting to bucket: {}", url);
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
		debug!("loading from bucket: {}", path);
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
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;


	#[sweet::test]
	async fn serves_fs() {
		let bucket = Bucket::new_test().await;
		let body = "body { color: red; }";
		let path = RoutePath::from("/style.css");
		bucket.insert(&path, body).await.unwrap();
		let response = super::bucket_to_response(&bucket, &path).await.unwrap();
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
	async fn html() {
		let bucket = Bucket::new_test().await;
		let body = "<h1>Hello, world!</h1>";
		bucket
			.insert(&RoutePath::from("/docs/index.html"), body)
			.await
			.unwrap();
		let response =
			super::bucket_to_response(&bucket, &RoutePath::from("/docs"))
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
		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![
				common_predicates::fallback(),
				BucketEndpoint::new(bucket.clone(), None),
			]))
			.oneshot_str(Request::get("/"))
			.await
			.xpect_str("<div>fallback</div>");
	}

	#[sweet::test]
	async fn remove_prefix() {
		let bucket = Bucket::new_test().await;
		let path = RoutePath::from("bar/index.html");
		bucket.insert(&path, "<div>fallback</div>").await.unwrap();
		FlowRouterPlugin::world()
			.spawn((RouteServer, Sequence, children![(
				PathFilter::new("foo"),
				children![(Sequence, children![
					common_predicates::fallback(),
					BucketEndpoint::new(
						bucket.clone(),
						Some(RoutePath::new("foo"))
					),
				])]
			)]))
			.oneshot_str("/foo/bar")
			.await
			.xpect_str("<div>fallback</div>");
	}
}
