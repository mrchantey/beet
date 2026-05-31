use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Configuration for the [`CorsHandler`] middleware, declaring which
/// origins may make cross-origin requests.
///
/// Spawn it alongside [`CorsHandler`] (see [`cors`]) on the router entity;
/// the middleware resolves it from the nearest ancestor.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct CorsConfig {
	/// Whether to allow requests from any origin.
	pub allow_any_origin: bool,
	/// Specific allowed origins, used when `allow_any_origin` is `false`.
	pub allowed_origins: Vec<String>,
}

impl CorsConfig {
	/// The wildcard value for `Access-Control-Allow-Origin`.
	pub const ANY_ORIGIN: &'static str = "*";

	/// Creates a config allowing any origin.
	pub fn allow_any() -> Self {
		Self {
			allow_any_origin: true,
			allowed_origins: vec![],
		}
	}

	/// Creates a config allowing only the given origins.
	pub fn allow_origins(
		origins: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		Self {
			allow_any_origin: false,
			allowed_origins: origins.into_iter().map(Into::into).collect(),
		}
	}

	/// Returns `true` if the given origin is allowed by this configuration.
	pub fn origin_allowed(&self, origin: &str) -> bool {
		self.allow_any_origin || self.allowed_origins.iter().any(|o| o == origin)
	}
}

/// Middleware that validates the request `Origin` against an ancestor
/// [`CorsConfig`] and sets CORS response headers.
///
/// `OPTIONS` requests short-circuit with a preflight response. A missing
/// origin (when not allowing any) yields `400`, a disallowed origin `403`.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn CorsHandler(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();

	let config = caller
		.with_state::<AncestorQuery<&CorsConfig>, CorsConfig>(
			move |entity, query| query.get(entity).cloned().unwrap_or_default(),
		)
		.await?;

	let origin_header = request
		.headers
		.get::<header::Origin>()
		.and_then(|res| res.ok());

	// resolve the origin to echo back, or short-circuit on a bad/blocked origin
	let origin = match (config.allow_any_origin, origin_header) {
		(true, Some(origin)) => origin,
		(true, None) => CorsConfig::ANY_ORIGIN.to_string(),
		(false, None) => {
			return Response::from_status_body(
				StatusCode::BAD_REQUEST,
				"Origin header not found",
				MediaType::Text,
			)
			.xok();
		}
		(false, Some(origin)) if config.origin_allowed(&origin) => origin,
		(false, Some(_)) => {
			return Response::from_status_body(
				StatusCode::FORBIDDEN,
				"Origin not allowed",
				MediaType::Text,
			)
			.xok();
		}
	};

	// OPTIONS preflight: respond immediately with the allow headers
	if *request.method() == HttpMethod::Options {
		let mut response = Response::ok();
		let headers = &mut response.parts.headers;
		headers.set::<header::AccessControlMaxAge>(60u32);
		headers
			.set::<header::AccessControlAllowHeaders>("content-type".to_string());
		headers.set::<header::AccessControlAllowOrigin>(origin);
		return Ok(response);
	}

	let mut response = next.call(request).await?;
	response
		.parts
		.headers
		.set::<header::AccessControlAllowOrigin>(origin);
	Ok(response)
}

/// Bundle combining the [`CorsHandler`] middleware with its [`CorsConfig`].
pub fn cors(config: CorsConfig) -> impl Bundle { (CorsHandler, config) }


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Hello(_cx: ActionContext<RequestParts>) -> MediaBytes {
		MediaBytes::new_text("Hello")
	}

	fn spawn_cors(world: &mut World, config: CorsConfig) -> Entity {
		world
			.spawn((default_router(children![exchange_route("", Hello)]), cors(config)))
			.flush()
	}

	#[beet_core::test]
	async fn allows_configured_origin() {
		let mut world = router_world();
		let root =
			spawn_cors(&mut world, CorsConfig::allow_origins(["https://allowed.com"]));
		let response = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::get("").with_header_raw("origin", "https://allowed.com"),
			)
			.await
			.unwrap();

		response.status().xpect_eq(StatusCode::OK);
		response
			.headers
			.get::<header::AccessControlAllowOrigin>()
			.unwrap()
			.unwrap()
			.xpect_eq("https://allowed.com");
	}

	#[beet_core::test]
	async fn blocks_disallowed_origin() {
		let mut world = router_world();
		let root = spawn_cors(&mut world, CorsConfig::allow_origins([] as [&str; 0]));
		world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::get("").with_header_raw("origin", "https://blocked.com"),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::FORBIDDEN);
	}

	#[beet_core::test]
	async fn allows_any_origin() {
		let mut world = router_world();
		let root = spawn_cors(&mut world, CorsConfig::allow_any());
		let response = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::get("").with_header_raw("origin", "https://anything.com"),
			)
			.await
			.unwrap();

		response.status().xpect_eq(StatusCode::OK);
		response
			.headers
			.get::<header::AccessControlAllowOrigin>()
			.unwrap()
			.unwrap()
			.xpect_eq("https://anything.com");
	}

	#[beet_core::test]
	async fn allow_any_without_origin_uses_wildcard() {
		let mut world = router_world();
		let root = spawn_cors(&mut world, CorsConfig::allow_any());
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.headers
			.get::<header::AccessControlAllowOrigin>()
			.unwrap()
			.unwrap()
			.xpect_eq("*");
	}

	#[beet_core::test]
	async fn preflight_returns_allow_headers() {
		let mut world = router_world();
		let root =
			spawn_cors(&mut world, CorsConfig::allow_origins(["https://allowed.com"]));
		let response = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::options("")
					.with_header_raw("origin", "https://allowed.com"),
			)
			.await
			.unwrap();

		response.status().xpect_eq(StatusCode::OK);
		response
			.headers
			.get::<header::AccessControlAllowOrigin>()
			.unwrap()
			.unwrap()
			.xpect_eq("https://allowed.com");
		response
			.headers
			.get::<header::AccessControlMaxAge>()
			.unwrap()
			.unwrap()
			.xpect_eq(60u32);
	}
}
