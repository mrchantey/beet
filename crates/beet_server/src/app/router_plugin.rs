use crate::app::bundle_layer_plugin::BundleLayerPlugin;
use crate::prelude::*;
use axum::extract::FromRequestParts;
use beet_net::prelude::*;


pub trait RouterPlugin: Sized {
	type State: 'static + Send + Sync + Clone;
	type Meta: 'static + Send + Sync + Clone;

	/// Layer this plugin with a [`BundleLayer`].
	fn layer<L>(self, layer: L) -> BundleLayerPlugin<L, Self>
	where
		Self: Sized,
	{
		BundleLayerPlugin::new(layer, self)
	}

	/// Whether routes provided by this plugin are static.
	/// Usually this should be `true` for pages and `false` for actions.
	fn is_static(&self) -> bool;

	/// List of routes that this plugin provides.
	fn routes(&self) -> Vec<RouteInfo>;

	/// List of metadata associated with each route.
	fn meta(&self) -> Vec<Self::Meta>;
	/// Returns a vector of tuples where each tuple contains a `RouteInfo` and its corresponding metadata.
	fn route_metas(&self) -> Vec<(RouteInfo, Self::Meta)> {
		// Combine routes and metas into a vector of tuples
		// where each tuple contains a RouteInfo and its corresponding Meta.
		self.routes()
			.into_iter()
			.zip(self.meta().into_iter())
			.collect()
	}

	/// Add routes to the provided router, overridden by middleware layers,
	/// which pass themselves to [`add_routes_with`](Self::add_routes_with).
	fn add_routes(&self, router: Router<Self::State>) -> Router<Self::State> {
		self.add_routes_with(router, self)
	}

	/// Allow for middleware routers to pass themselves in
	fn add_routes_with(
		&self,
		router: Router<Self::State>,
		plugin: &impl RouterPlugin<State = Self::State, Meta = Self::Meta>,
	) -> Router<Self::State>;

	/// Call this method instead of [`add_routes`](Self::add_routes) to add routes which can
	/// be layered by a [`BundleLayer`].
	fn add_bundle_route<M, H>(
		&self,
		router: Router<Self::State>,
		route_info: RouteInfo,
		handler: H,
		_meta: Self::Meta,
	) -> Router<Self::State>
	where
		H: BundleRoute<M, State = Self::State>,
		H::Extractors: 'static + Send + Sync + FromRequestParts<Self::State>,
	{
		// default to just calling `add_route`,
		// but middleware layers can override this with a BundleLayer
		self.add_route(router, route_info, handler)
	}
	fn add_route<M>(
		&self,
		router: Router<Self::State>,
		route_info: RouteInfo,
		handler: impl IntoBeetRoute<M, State = Self::State>,
	) -> Router<Self::State> {
		handler.add_beet_route(router, route_info)
	}
}


pub trait IntoRoutePlugins<S, M> {
	fn add_to_router(self, router: &mut AppRouter<S>);
}

impl<T: RouterPlugin> IntoRoutePlugins<T::State, T> for T {
	fn add_to_router(self, app_router: &mut AppRouter<T::State>) {
		if self.is_static() {
			app_router.static_routes.extend(self.routes());
			app_router.router = self.add_routes_with(
				app_router.router.clone(),
				&ClientIslandPlugin::<T>::default(),
			);
		}
		app_router.router = self.add_routes(app_router.router.clone());
	}
}


use variadics_please::all_tuples;

pub struct RouterPluginsTupleMarker;

macro_rules! impl_router_plugins_tuples {
        ($(#[$meta:meta])* $(($param: ident, $plugins: ident)),*) => {
            $(#[$meta])*
            impl<S,$($param, $plugins),*> IntoRoutePlugins<S,(RouterPluginsTupleMarker, $($param,)*)> for ($($plugins,)*)
            where
                $($plugins: IntoRoutePlugins<S,$param>),*
            {
                #[expect(
                    clippy::allow_attributes,
                    reason = "This is inside a macro, and as such, may not trigger in all cases."
                )]
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                #[allow(unused_variables, reason = "`app` is unused when implemented for the unit type `()`.")]
                #[track_caller]
                fn add_to_router(self, router: &mut AppRouter<S>) {
                    let ($($plugins,)*) = self;
                    $($plugins.add_to_router(router);)*
                }
            }
        }
    }

all_tuples!(impl_router_plugins_tuples, 1, 15, M, T);


#[cfg(test)]
mod test {
	use crate::prelude::*;

	struct Plugin1;
	impl RouterPlugin for Plugin1 {
		type State = ();
		type Meta = ();
		fn is_static(&self) -> bool { unimplemented!() }
		fn routes(&self) -> Vec<RouteInfo> { unimplemented!() }
		fn meta(&self) -> Vec<Self::Meta> { unimplemented!() }
		fn add_routes_with(
			&self,
			_: Router<Self::State>,
			_: &impl RouterPlugin<State = Self::State, Meta = Self::Meta>,
		) -> Router<Self::State> {
			unimplemented!()
		}
	}
	struct Plugin2;
	impl RouterPlugin for Plugin2 {
		type State = ();
		type Meta = ();
		fn is_static(&self) -> bool { unimplemented!() }
		fn routes(&self) -> Vec<RouteInfo> { unimplemented!() }
		fn meta(&self) -> Vec<Self::Meta> { unimplemented!() }
		fn add_routes_with(
			&self,
			_: Router<Self::State>,
			_: &impl RouterPlugin<State = Self::State, Meta = Self::Meta>,
		) -> Router<Self::State> {
			unimplemented!()
		}
	}

	#[test]
	fn works() {
		fn foo<M>(_: impl IntoRoutePlugins<(), M>) {}
		foo((Plugin1, Plugin2));
	}
}
