use crate::prelude::*;
use axum::extract::FromRequestParts;
use axum::routing::MethodFilter;
use axum::routing::{
	self,
};
use beet_net::prelude::*;
use bevy::prelude::*;

/// Methods that accept a tuple of extractors and return a bundle.
pub trait BundleRoute<M>: 'static + Send + Sync + Clone {
	type Bundle: Bundle;
	type State: 'static + Send + Sync + Clone;
	type Extractors: 'static + Send + FromRequestParts<Self::State>;
	fn into_bundle_result(
		self,
		extractors: Self::Extractors,
	) -> impl 'static + Send + Future<Output = AppResult<Self::Bundle>>;
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

pub struct BundleRouteIntoBeetRouteMarker;

impl<R, M> IntoBeetRoute<(BundleRouteIntoBeetRouteMarker, M)> for R
where
	R: BundleRoute<M>,
{
	type State = R::State;
	fn add_beet_route(
		self,
		router: Router<Self::State>,
		route_info: RouteInfo,
	) -> Router<Self::State> {
		router.route(
			&route_info.path.to_string_lossy(),
		routing::on(
			route_info.method.into_axum_method(),
			async move |extractors: R::Extractors| -> AppResult<BundleResponse<R::Bundle>> {
				let bundle = self.into_bundle_result(extractors).await?;
				Ok(BundleResponse::new(bundle))
			},
		)		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::extract::Query as QueryParams;
	use beet_common::prelude::*;
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
		// this machinery is usually done by the AppRouter

		use axum::routing::get;
		let router: Router = Router::new().route(
			"/test",
			get(async move |e| BundleResponse::new(my_route(e))),
		);
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
			.to_be("<!DOCTYPE html><html><head></head><body><body><h1>hello world!</h1></body></body></html>");
	}
}
