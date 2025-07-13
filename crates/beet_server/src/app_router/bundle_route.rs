use crate::prelude::*;
use axum::extract::FromRequestParts;
use axum::extract::State;
use axum::response::Html;
use axum::routing::MethodFilter;
use axum::routing::{
	self,
};
use beet_core::prelude::*;
use bevy::prelude::*;

/// Methods that accept a tuple of extractors and return a bundle.
pub trait BundleRoute<M>: 'static + Send + Sync + Clone {
	type Bundle: Bundle;
	type State: DerivedAppState;
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
				async move |state: State<Self::State>,
				            extractors: R::Extractors|
				            -> AppResult<Html<String>> {
					let bundle = self.into_bundle_result(extractors).await?;
					Ok(state.render_bundle(bundle))
				},
			),
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::Router;
	use axum::extract::Query as QueryParams;
	use beet_core::prelude::*;
	use beet_rsx::prelude::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	use sweet::prelude::*;

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
		use axum::extract::State;
		use axum::routing::get;
		let mut router: Router = Router::new()
			.route(
				"/test",
				get(async move |state: State<AppRouterState>, e| {
					state.render_bundle(my_route(e))
				}),
			)
			.with_state(AppRouterState::test());
		router.oneshot_str("/test?name=world").await.unwrap().xpect()
			.to_be("<!DOCTYPE html><html><head></head><body><body><h1>hello world!</h1></body></body></html>");
	}
}
