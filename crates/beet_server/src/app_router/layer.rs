use crate::prelude::*;
use axum::extract::FromRequestParts;

pub trait Layer<M> {
	type Extractors;
	type State;
	type Meta;
	type Inner;
	type Output;

	fn call_layer(
		&self,
		inner: Self::Inner,
		meta: Self::Meta,
		extractors: Self::Extractors,
	) -> impl Send + Future<Output = Self::Output>;
}

impl Layer<Self> for () {
	type Extractors = ();
	type State = ();
	type Meta = ();
	type Inner = ();
	type Output = ();

	fn call_layer(
		&self,
		_inner: Self::Inner,
		_meta: Self::Meta,
		_extractors: Self::Extractors,
	) -> impl Send + Future<Output = Self::Output> {
		async { () }
	}
}



pub struct FuncLayerMarker;
pub struct AsyncFuncLayerMarker;

// Blanket impl for Layer for functions
impl<Extractors, State, Meta, Inner, Output, Func>
	Layer<(Extractors, State, Meta, Inner, Output, FuncLayerMarker)> for Func
where
	Func: Send + Sync + Fn(Inner, Meta, Extractors) -> Output,
	Extractors: Send,
	State: Send,
	Inner: Send,
	Meta: Send,
	Output: Send,
{
	type Extractors = Extractors;
	type State = State;
	type Meta = Meta;
	type Inner = Inner;
	type Output = Output;

	fn call_layer(
		&self,
		inner: Self::Inner,
		meta: Self::Meta,
		extractors: Self::Extractors,
	) -> impl Send + Future<Output = Self::Output> {
		async move { (self)(inner, meta, extractors) }
	}
}
// Blanket impl for Layer for async functions
impl<Extractors, State, Meta, Inner, Output, Func, Fut>
	Layer<(
		Extractors,
		State,
		Meta,
		Inner,
		Output,
		Fut,
		AsyncFuncLayerMarker,
	)> for Func
where
	Func: Send + Sync + Fn(Inner, Meta, Extractors) -> Fut,
	Fut: Send + Future<Output = Output>,
	Extractors: Send,
	State: Send,
	Inner: Send,
	Meta: Send,
	Output: Send,
{
	type Extractors = Extractors;
	type State = State;
	type Meta = Meta;
	type Inner = Inner;
	type Output = Output;

	fn call_layer(
		&self,
		inner: Self::Inner,
		meta: Self::Meta,
		extractors: Self::Extractors,
	) -> impl Send + Future<Output = Self::Output> {
		(self)(inner, meta, extractors)
	}
}


/// Wraps a [`BundleRoute`] with a [`BundleLayerHandler`].
#[derive(Debug, Clone)]
pub struct BundleLayer<L, R, M> {
	/// The [`BundleLayerHandler`]
	layer: L,
	/// The inner [`BundleRoute`]
	route: R,
	/// The metadata for this [`BundleRoute`]
	meta: M,
}

impl<L, R, M> BundleLayer<L, R, M> {
	pub fn new(route: R, meta: M, layer: L) -> Self {
		Self { layer, route, meta }
	}
}

/// Implement [`Route`] for any layer where the [`Route::Output`] can be
/// directly fed to the [`Layer::Inner`]
impl<LayerT, LayerM, RouteT, Meta, RouteMarker> Route<(LayerM, RouteMarker)>
	for BundleLayer<LayerT, RouteT, Meta>
where
	LayerT: Send + Layer<LayerM, Meta = Meta>,
	LayerT::Extractors: Send,
	RouteT::Extractors: Send,
	RouteT: Send
		+ Route<RouteMarker, State = LayerT::State, Output = LayerT::Inner>,
	Meta: Send + Sync + Clone,
	(LayerT::Extractors, RouteT::Extractors): Send,
{
	type State = LayerT::State;
	type Extractors = (LayerT::Extractors, RouteT::Extractors);
	type Output = LayerT::Output;

	fn call_route(
		self,
		(layer_extractors, route_extractors): Self::Extractors,
	) -> impl Send + Future<Output = Self::Output> {
		async move {
			let inner = self.route.call_route(route_extractors).await;
			self.layer
				.call_layer(inner, self.meta.clone(), layer_extractors)
				.await
		}
	}
}

/// Implement [`Route`] for any layer where the [`Route::Output`] is
/// a result and the [`Ok`] can be fed to the [`Layer::Inner`]
impl<LayerT, LayerM, RouteT, M, Err, RouteMarker>
	Route<(LayerM, Self, Err, RouteMarker)> for BundleLayer<LayerT, RouteT, M>
where
	LayerT: Send + Layer<LayerM, Meta = M>,
	LayerT::Extractors: Send,
	RouteT::Extractors: Send,
	RouteT: Send + Route<RouteMarker, Output = Result<LayerT::Inner, Err>>,
	M: Send + Clone,
	(LayerT::Extractors, RouteT::Extractors):
		Send + FromRequestParts<LayerT::State>,
{
	type State = LayerT::State;
	type Extractors = (LayerT::Extractors, RouteT::Extractors);
	type Output = Result<LayerT::Output, Err>;

	fn call_route(
		self,
		(layer_extractors, route_extractors): Self::Extractors,
	) -> impl Send + Future<Output = Self::Output> {
		async move {
			let inner = self.route.call_route(route_extractors).await?;
			let out = self
				.layer
				.call_layer(inner, self.meta.clone(), layer_extractors)
				.await;
			Ok(out)
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use axum::extract::Query as QueryParams;
	use bevy::prelude::*;
	use serde::Deserialize;
	use sweet::prelude::*;

	#[derive(Debug, Clone, Deserialize)]
	struct BundleParams {
		occupation: String,
	}
	#[derive(Debug, Clone, Deserialize)]
	struct LayerParams {
		name: String,
	}

	#[derive(Debug, Clone, Default)]
	struct MyMiddleware;


	fn my_middleware(
		bundle: impl Bundle,
		_meta: (),
		params: QueryParams<LayerParams>,
	) -> impl Bundle {
		rsx! {
			<div>
				<span>name: {params.name.clone()}</span>
				{bundle}
			</div>
		}
	}

	fn my_bundle_route(params: QueryParams<BundleParams>) -> impl Bundle {
		rsx! {<span>occupation: {params.occupation.clone()}</span>}
	}

	// #[sweet::test]
	// async fn works() {
	// 	fn foo<M>(_: impl Layer<M>) {}
	// 	foo(my_middleware);
	// 	fn bar<RM, LM>(_: impl Route<RM>, _: impl Layer<LM>) {}
	// 	bar(my_bundle_route, my_middleware);
	// 	fn bazz<RM>(_: impl Route<RM>) {}
	// 	bazz(BundleLayer::new(my_middleware, my_bundle_route, ()));

	// 	AppRouter::test()
	// 		.add_route("/", BundleLayer::new(MyMiddleware, my_bundle_route,()))
	// 		.render_route(&"/?name=pizzaguy&occupation=delivermepizza".into())
	// 		.await
	// 		.unwrap()
	// 		.xpect()
	// 		.to_be_str("<!DOCTYPE html><html><head></head><body><div><span>name: pizzaguy</span><span>occupation: delivermepizza</span></div></body></html>");
	// }
}
