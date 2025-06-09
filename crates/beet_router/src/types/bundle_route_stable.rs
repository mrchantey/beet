use crate::prelude::*;
use axum::extract::FromRequestParts;
use bevy::prelude::*;
use std::pin::Pin;

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
