use crate::prelude::*;
use axum::extract::FromRequestParts;
use axum::handler::Handler;
use axum::routing;
use axum::routing::MethodRouter;
use beet_rsx::html::bundle_to_html;
use bevy::prelude::*;
use std::convert::Infallible;
use std::pin::Pin;

/// Methods that accept a tuple of extractors and return a bundle.
pub trait BundleRoute<M>: 'static + Send + Sync + Clone {
	type Bundle: Bundle;
	type State: 'static + Send + Sync + Clone;
	type Extractors: 'static + Send + FromRequestParts<Self::State>;
	type Future: Future<Output = AppResult<Self::Bundle>> + Send + 'static;
	fn into_bundle_result(self, extractors: Self::Extractors) -> Self::Future;

	fn into_method_router(
		self,
		method: HttpMethod,
	) -> MethodRouter<Self::State, Infallible>
	where
		Self: Sized,
	{
		method_router(
			method,
			async move |extractors: Self::Extractors| -> AppResult<String> {
				let bundle = self.into_bundle_result(extractors).await?;
				let html = bundle_to_html(bundle);
				Ok(html)
			},
		)
	}
}

fn method_router<H, T, S>(
	method: HttpMethod,
	handler: H,
) -> MethodRouter<S, Infallible>
where
	H: Handler<T, S>,
	T: 'static,
	S: Clone + Send + Sync + 'static,
{
	let func = match method {
		HttpMethod::Get => routing::get,
		HttpMethod::Post => routing::post,
		HttpMethod::Put => routing::put,
		HttpMethod::Patch => routing::patch,
		HttpMethod::Delete => routing::delete,
		HttpMethod::Options => routing::options,
		HttpMethod::Head => routing::head,
		HttpMethod::Trace => routing::trace,
		HttpMethod::Connect => routing::connect,
	};
	func(handler)
}


pub struct BundleRouteToAsyncResultMarker;
pub struct BundleRouteToAsyncBundleMarker;
pub struct BundleRouteToBundleMarker;
pub struct BundleRouteToResultMarker;

impl<E, B, S, Func, Fut> BundleRoute<(E, B, S, BundleRouteToAsyncResultMarker)>
	for Func
where
	E: 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: 'static + Send + Sync + Clone,
	Func: 'static + Send + Sync + Clone + Fn(E) -> Fut,
	Fut: Future<Output = AppResult<B>> + Send + 'static,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;
	type Future = Fut;
	fn into_bundle_result(self, extractors: E) -> Self::Future {
		self(extractors)
	}
}

impl<E, B, S, Func, Fut> BundleRoute<(B, E, S, BundleRouteToAsyncBundleMarker)>
	for Func
where
	E: 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: 'static + Send + Sync + Clone,
	Func: 'static + Send + Sync + Clone + Fn(E) -> Fut,
	Fut: Future<Output = B> + Send + 'static,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;
	type Future = Pin<Box<dyn Future<Output = AppResult<B>> + Send + 'static>>;
	fn into_bundle_result(self, extractors: E) -> Self::Future {
		Box::pin(async move { Ok(self(extractors).await) })
	}
}

impl<E, B, S, Func> BundleRoute<(B, E, S, BundleRouteToBundleMarker)> for Func
where
	E: 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: 'static + Send + Sync + Clone,
	Func: 'static + Send + Sync + Clone + Fn(E) -> B,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;
	type Future = Pin<Box<dyn Future<Output = AppResult<B>> + Send + 'static>>;
	fn into_bundle_result(self, extractors: E) -> Self::Future {
		Box::pin(async move { Ok(self(extractors)) })
	}
}

impl<E, B, S, Func> BundleRoute<(B, E, S, BundleRouteToResultMarker)> for Func
where
	E: 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: 'static + Send + Sync + Clone,
	Func: 'static + Send + Sync + Clone + Fn(E) -> AppResult<B>,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;
	type Future = Pin<Box<dyn Future<Output = AppResult<B>> + Send + 'static>>;
	fn into_bundle_result(self, extractors: E) -> Self::Future {
		Box::pin(async move { self(extractors) })
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::extract::Query as QueryParams;
	use beet_common::prelude::*;
	use beet_rsx::prelude::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	use sweet::prelude::*;
	use tower::util::ServiceExt;

	#[derive(Deserialize)]
	struct RequestPayload {
		name: String,
	}


	fn my_route(
		// System Input, if any, is a tuple of axum extractors
		payload: QueryParams<RequestPayload>,
		// otherwise its a regular system
		// query: Query<&Name>,
		// world: &mut World,
	) -> impl Bundle {
		let name = payload.name.clone();
		rsx! {
			<body>
				<h1>hello {name}!</h1>
				// <p>time: {time.elapsed_secs()}</p>
			</body>
		}
	}

	#[sweet::test]
	async fn works() {
		let router: Router = Router::new()
			.route("/test", my_route.into_method_router(HttpMethod::Get));
		let response = router
			.oneshot(
				axum::http::Request::builder()
					.uri("/test?name=world")
					.body(axum::body::Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		String::from_utf8(body.to_vec())
			.unwrap()
			.xpect()
			.to_be("<body><h1>hello world!</h1></body>");
	}
}
