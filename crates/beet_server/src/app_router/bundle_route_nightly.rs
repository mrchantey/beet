//! Implement BundleRoute for functions that have any number of extractors
use crate::prelude::*;
use axum::extract::FromRequestParts;
use bevy::prelude::*;
use std::marker::Tuple;

pub struct BundleRouteToAsyncResultMarker;
pub struct BundleRouteToAsyncBundleMarker;
pub struct BundleRouteToBundleMarker;
pub struct BundleRouteToResultMarker;

impl<E, B, S, Func, Fut> BundleRoute<(E, B, S, BundleRouteToAsyncResultMarker)>
	for Func
where
	E: Tuple + 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: DerivedAppState,
	Func: 'static + Send + Sync + Clone + Fn<E, Output = Fut>,
	Fut: Future<Output = AppResult<B>> + Send + 'static,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;

	fn into_bundle_result(
		self,
		extractors: E,
	) -> impl 'static + Send + Future<Output = AppResult<Self::Bundle>> {
		self.call(extractors)
	}
}

impl<E, B, S, Func, Fut> BundleRoute<(B, E, S, BundleRouteToAsyncBundleMarker)>
	for Func
where
	E: Tuple + 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: DerivedAppState,
	Func: 'static + Send + Sync + Clone + Fn<E, Output = Fut>,
	Fut: Future<Output = B> + Send + 'static,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;

	fn into_bundle_result(
		self,
		extractors: E,
	) -> impl 'static + Send + Future<Output = AppResult<Self::Bundle>> {
		async move { Ok(self.call(extractors).await) }
	}
}

impl<E, B, S, Func> BundleRoute<(B, E, S, BundleRouteToBundleMarker)> for Func
where
	E: Tuple + 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: DerivedAppState,
	Func: 'static + Send + Sync + Clone + Fn<E, Output = B>,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;

	fn into_bundle_result(
		self,
		extractors: E,
	) -> impl 'static + Send + Future<Output = AppResult<Self::Bundle>> {
		async move { Ok(self.call(extractors)) }
	}
}

impl<E, B, S, Func> BundleRoute<(B, E, S, BundleRouteToResultMarker)> for Func
where
	E: Tuple + 'static + Send + FromRequestParts<S>,
	B: Bundle,
	S: DerivedAppState,
	Func: 'static + Send + Sync + Clone + Fn<E, Output = AppResult<B>>,
{
	type Bundle = B;
	type Extractors = E;
	type State = S;

	fn into_bundle_result(
		self,
		extractors: E,
	) -> impl 'static + Send + Future<Output = AppResult<Self::Bundle>> {
		async move { self.call(extractors) }
	}
}
