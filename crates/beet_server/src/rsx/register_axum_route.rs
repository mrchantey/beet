use axum::Router;
use axum::handler::Handler;
use axum::routing;
use beet_router::prelude::*;
use http::Method;


pub type RegisterAxumRoute<S> =
	Box<dyn FnOnce(Router<S>) -> Router<S> + Send + Sync>;


pub struct HandlerToRouteFuncMarker;

pub trait IntoRegisterAxumRoute<S, M> {
	fn into_register_axum_route(
		self,
		route_info: &RouteInfo,
	) -> RegisterAxumRoute<S>;
}

impl<H, T, S> IntoRegisterAxumRoute<S, (HandlerToRouteFuncMarker, H, T, S)>
	for H
where
	H: Handler<T, S>,
	T: 'static,
	S: Clone + Send + Sync + 'static,
{
	fn into_register_axum_route(
		self,
		route_info: &RouteInfo,
	) -> RegisterAxumRoute<S> {
		let func = if route_info.method == Method::GET {
			routing::get
		} else if route_info.method == Method::POST {
			routing::post
		} else if route_info.method == Method::PUT {
			routing::put
		} else if route_info.method == Method::DELETE {
			routing::delete
		} else if route_info.method == Method::PATCH {
			routing::patch
		} else {
			panic!("Unsupported method: {}", route_info.method)
		};
		let path = route_info.path.to_string();
		Box::new(move |router: Router<S>| router.route(&path, func(self)))
	}
}

// pub fn $name<H, T, S>(handler: H) -> MethodRouter<S, Infallible>
// where
// 		H: Handler<T, S>,
// 		T: 'static,
// 		S: Clone + Send + Sync + 'static,
// {
// 		on(MethodFilter::$method, handler)
// }




// impl<F:RsxR
