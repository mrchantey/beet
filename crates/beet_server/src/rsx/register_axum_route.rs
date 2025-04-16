use axum::Router;
use axum::handler::Handler;
use axum::routing;
use beet_router::prelude::*;

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
		let func = match route_info.method {
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
