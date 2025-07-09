use crate::prelude::*;
use axum::Router;
use axum::extract::FromRequestParts;
use axum::extract::State;
use axum::routing;
use beet_core::exports::ron;
use beet_router::prelude::ClientIsland;
use std::marker::PhantomData;


pub struct ClientIslandRouterPlugin<T> {
	phantom: PhantomData<T>,
}
impl<T> Default for ClientIslandRouterPlugin<T> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

impl ClientIslandRouterPlugin<()> {
	pub fn route_info(route_info: &RouteInfo) -> RouteInfo {
		let path = format!("/__client_islands{}", route_info.path);
		RouteInfo::new(path, route_info.method)
	}
}

// impl<'a, T> ClientIslandPlugin<'a, T> {
// 	pub fn new(router_plugin: &'a T) -> Self { Self { router_plugin } }
// }


impl<T> RouterPlugin for ClientIslandRouterPlugin<T>
where
	T: RouterPlugin,
{
	type Meta = T::Meta;
	type State = T::State;

	fn is_static(&self) -> bool {
		// this router outputs ron not html?
		unimplemented!("ClientIslandPlugin should not be used directly")
	}

	fn routes(&self) -> Vec<beet_core::prelude::RouteInfo> {
		unimplemented!("ClientIslandPlugin should not be used directly")
	}

	fn meta(&self) -> Vec<Self::Meta> {
		unimplemented!("ClientIslandPlugin should not be used directly")
	}

	fn add_routes_with(
		&self,
		_router: Router<Self::State>,
		_plugin: &impl RouterPlugin<State = Self::State, Meta = Self::Meta>,
	) -> Router<Self::State> {
		unimplemented!("ClientIslandPlugin should not be used directly")
	}
	fn add_bundle_route<M, H>(
		&self,
		router: Router<Self::State>,
		route_info: beet_core::prelude::RouteInfo,
		handler: H,
		_meta: Self::Meta,
	) -> Router<Self::State>
	where
		H: BundleRoute<M, State = Self::State>,
		H::Extractors: 'static + Send + Sync + FromRequestParts<Self::State>,
	{
		let route_info = ClientIslandRouterPlugin::route_info(&route_info);
		router.route(
			&route_info.path.to_string_lossy(),
			routing::on(
				route_info.method.into_axum_method(),
				async move |state: State<Self::State>,
				            e|
				            -> AppResult<String> {
					let bundle = handler.into_bundle_result(e).await?;
					let islands =
						ClientIsland::collect(&mut state.create_app(), bundle)
							.map_err(|e| {
								AppError::internal_error(format!(
									"Failed to collect client islands: {}",
									e
								))
							})?;
					let islands =
						ron::ser::to_string(&islands).map_err(|e| {
							AppError::internal_error(format!(
								"Failed to serialize client islands: {}",
								e
							))
						})?;

					Ok(islands)
				},
			),
		)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::prelude::*;
	use beet_rsx::as_beet::*;
	use bevy::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	#[template]
	#[derive(Serialize, Deserialize)]
	pub fn MyTemplate(foo: u32) -> impl Bundle {
		let _ = foo;
		()
	}

	fn route_info() -> RouteInfo {
		RouteInfo::new("/my_route".to_string(), HttpMethod::Get)
	}

	struct Plugin1;
	impl RouterPlugin for Plugin1 {
		type State = AppRouterState;
		type Meta = ();
		fn is_static(&self) -> bool { true }
		fn routes(&self) -> Vec<RouteInfo> { vec![route_info()] }
		fn meta(&self) -> Vec<Self::Meta> { vec![()] }
		fn add_routes_with(
			&self,
			router: Router<Self::State>,
			plugin: &impl RouterPlugin<State = Self::State, Meta = Self::Meta>,
		) -> Router<Self::State> {
			plugin.add_bundle_route(
				router,
				route_info(),
				|| rsx! {<MyTemplate foo=3 client:load/>},
				(),
			)
		}
	}

	#[sweet::test]
	async fn works() {
		AppRouter::test()
			.add_plugins(Plugin1)
			.get_client_islands(&route_info())
			.await
			.unwrap()
			.xpect()
			.to_be(vec![ClientIsland {
				template: TemplateSerde::new(&MyTemplate { foo: 3 }),
				dom_idx: DomIdx(0),
				mount: false,
			}]);
	}
}
