use crate::prelude::*;
use axum::Router;
use axum::extract::FromRequestParts;
use axum::response::Html;
use axum::routing;
use axum::routing::MethodFilter;
use axum::routing::MethodRouter;
use beet_net::prelude::*;
use beet_template::prelude::*;
use bevy::prelude::*;
use std::convert::Infallible;

/// Methods that accept a tuple of extractors and return a bundle.
pub trait BundleRoute<M>: 'static + Send + Sync + Clone {
	type Bundle: Bundle;
	type State: 'static + Send + Sync + Clone;
	type Extractors: 'static + Send + FromRequestParts<Self::State>;
	type Future: Future<Output = AppResult<Self::Bundle>> + Send + 'static;
	fn into_bundle_result(self, extractors: Self::Extractors) -> Self::Future;

	/// Converts the route into a method router that can be used in an axum router.
	fn into_method_router(
		self,
		method: HttpMethod,
	) -> MethodRouter<Self::State, Infallible>
	where
		Self: Sized,
	{
		routing::on(
			method.into_axum_method(),
			async move |extractors: Self::Extractors| -> AppResult<Html<String>> {
				let bundle = self.into_bundle_result(extractors).await?;
				let html = HtmlFragment::parse_bundle(bundle);
				Ok(Html(html))
			},
		)
	}
}
#[extend::ext(name=RouterBundleRouteExt)]
pub impl<S> Router<S>
where
	S: 'static + Send + Sync + Clone,
{
	fn bundle_route<R, M>(
		self,
		info: impl Into<RouteInfo>,
		route: R,
	) -> Router<S>
	where
		R: BundleRoute<M, State = S>,
	{
		let info = info.into();
		self.route(
			&info.path.to_string(),
			route.into_method_router(info.method),
		)
	}
}


#[extend::ext(name=HttpMethodExt)]
pub impl HttpMethod {
	fn into_axum_method(&self) -> MethodFilter {
		match self {
			HttpMethod::Get => MethodFilter::GET,
			HttpMethod::Post => MethodFilter::POST,
			HttpMethod::Put => MethodFilter::PUT,
			HttpMethod::Patch => MethodFilter::PATCH,
			HttpMethod::Delete => MethodFilter::DELETE,
			HttpMethod::Options => MethodFilter::OPTIONS,
			HttpMethod::Head => MethodFilter::HEAD,
			HttpMethod::Trace => MethodFilter::TRACE,
			HttpMethod::Connect => MethodFilter::CONNECT,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::extract::Query as QueryParams;
	use beet_common::prelude::*;
	use beet_net::prelude::*;
	use beet_template::prelude::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	use sweet::prelude::*;
	use tower::util::ServiceExt;

	#[derive(Deserialize)]
	struct RequestPayload {
		name: String,
	}


	fn my_route(payload: QueryParams<RequestPayload>) -> impl Bundle {
		let name = payload.name.clone();
		rsx! {
			<body>
				<h1>hello {name}!</h1>
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
