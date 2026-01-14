//! Common middleware actions for HTTP requests and responses.
//!
//! Unlike predicates which gate execution, middleware modifies requests or responses
//! and typically triggers [`Outcome::Pass`] to continue the flow.
//!
//! ## Request vs Response Middleware
//!
//! **Response middleware** runs **after** endpoints in the behavior tree, modifying the
//! [`Response`] component that was inserted by the endpoint handler. Use these with
//! [`InfallibleSequence`] to ensure all middleware runs.
//!
//! **Request middleware** runs **before** endpoints, validating and storing information
//! from the [`Request`] before it is consumed by the endpoint handler.
//!
//! ## Pattern: Response Middleware
//!
//! Response middleware like [`no_cache_headers`] should run after endpoints:
//!
//! ```
//! # use beet_router::prelude::*;
//! # use beet_core::prelude::*;
//! # use beet_flow::prelude::*;
//! # use beet_net::prelude::*;
//! ExchangeSpawner::new_flow(|| {
//!     (InfallibleSequence, children![
//!         EndpointBuilder::get().with_handler(|| "Hello"),
//!         common_middleware::no_cache_headers(),
//!     ])
//! });
//! ```
//!
//! ## Pattern: Request + Response Middleware (CORS)
//!
//! CORS requires both request-phase validation and response-phase header insertion:
//!
//! ```
//! # use beet_router::prelude::*;
//! # use beet_core::prelude::*;
//! # use beet_flow::prelude::*;
//! # use beet_net::prelude::*;
//! let config = CorsConfig::new(true, vec![]);
//! ExchangeSpawner::new_flow(move || {
//!     (InfallibleSequence, children![
//!         // Request phase: validate origin, store in ValidatedOrigin component
//!         common_middleware::cors_request(config.clone()),
//!         // Endpoint handles the request
//!         EndpointBuilder::get().with_handler(|| "Hello"),
//!         // Response phase: add CORS headers from ValidatedOrigin
//!         common_middleware::cors_response(config),
//!     ])
//! });
//! ```
//!
//! ## Pattern: CORS Preflight
//!
//! For OPTIONS preflight requests, use [`Fallback`] so the endpoint only runs if
//! preflight didn't handle the request:
//!
//! ```
//! # use beet_router::prelude::*;
//! # use beet_core::prelude::*;
//! # use beet_flow::prelude::*;
//! # use beet_net::prelude::*;
//! let config = CorsConfig::new(true, vec![]);
//! ExchangeSpawner::new_flow(move || {
//!     (Fallback, children![
//!         // Handle OPTIONS preflight and return early
//!         common_middleware::cors_preflight(config.clone()),
//!         // Endpoint only runs if not OPTIONS
//!         EndpointBuilder::any_method().with_handler(|| "Hello"),
//!     ])
//! });
//! ```

use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

/// Add no-cache headers to a response.
///
/// This middleware modifies the Response component on the agent entity if one exists.
/// If no Response exists, it triggers [`Outcome::Fail`].
///
/// # Example
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_net::prelude::*;
/// ExchangeSpawner::new_flow(|| {
///     (InfallibleSequence, children![
///         EndpointBuilder::get().with_handler(|| "Hello"),
///         common_middleware::no_cache_headers(),
///     ])
/// });
/// ```
pub fn no_cache_headers() -> impl Bundle {
	(
		Name::new("No-Cache Headers Middleware"),
		OnSpawn::observe(
			|ev: On<GetOutcome>, agents: AgentQuery, mut commands: Commands| {
				let action = ev.target();
				let agent = agents.entity(action);

				commands.queue(move |world: &mut World| -> Result {
					let mut entity = world.entity_mut(agent);
					let Some(mut response) = entity.get_mut::<Response>()
					else {
						cross_log!(
							"No Response found for no_cache_headers middleware"
						);
						return Ok(());
					};

					let parts = response.parts_mut();
					parts.insert_header(
						"cache-control",
						"no-cache, no-store, must-revalidate",
					);
					parts.insert_header("pragma", "no-cache");
					parts.insert_header("expires", "0");
					Ok(())
				});

				commands.entity(action).trigger_target(Outcome::Pass);
			},
		),
	)
}

/// Configuration for CORS middleware
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct CorsConfig {
	pub allow_any_origin: bool,
	allowed_origins: Vec<String>,
}

impl CorsConfig {
	pub const ANY_ORIGIN: &'static str = "*";

	pub fn new(
		allow_any_origin: bool,
		allowed_origins: Vec<&'static str>,
	) -> Self {
		Self {
			allow_any_origin,
			allowed_origins: allowed_origins
				.into_iter()
				.map(|s| s.to_string())
				.collect(),
		}
	}

	pub fn origin_allowed(&self, origin: &str) -> bool {
		self.allow_any_origin
			|| self.allowed_origins.iter().any(|o| o == origin)
	}
}

/// Component that stores the validated CORS origin for later use
#[derive(Debug, Clone, Component)]
pub struct ValidatedOrigin(pub String);

/// Request-phase CORS middleware that validates origin and stores it.
///
/// This middleware:
/// - Reads the Request component to get the origin header
/// - Validates the origin against the CorsConfig
/// - Stores the validated origin in a [`ValidatedOrigin`] component
/// - Returns an error response and triggers [`Outcome::Fail`] if validation fails
///
/// Should be paired with [`cors_response`] to add CORS headers to the response.
///
/// # Example
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_net::prelude::*;
/// let config = CorsConfig::new(false, vec!["https://example.com"]);
/// ExchangeSpawner::new_flow(|| {
///     (InfallibleSequence, children![
///         common_middleware::cors_request(config.clone()),
///         EndpointBuilder::get().with_handler(|| "Hello"),
///         common_middleware::cors_response(config),
///     ])
/// });
/// ```
pub fn cors_request(config: CorsConfig) -> impl Bundle {
	(
		Name::new("CORS Request Middleware"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      agents: AgentQuery,
			      mut commands: Commands| {
				let action = ev.target();
				let agent = agents.entity(action);
				let config = config.clone();

				commands.queue(move |world: &mut World| -> Result {
					// Get request to read origin header
					let origin_header = world
						.entity(agent)
						.get::<Request>()
						.ok_or_else(|| {
							bevyhow!("No Request found for CORS middleware")
						})?
						.get_header("origin")
						.map(|s| s.to_string());

					let origin = match (config.allow_any_origin, origin_header)
					{
						(true, None) => CorsConfig::ANY_ORIGIN.to_string(),
						(true, Some(origin)) => origin,
						(false, None) => {
							world.entity_mut(agent).insert(
								Response::from_status_body(
									StatusCode::MalformedRequest,
									b"Origin header not found",
									"text/plain",
								),
							);
							world
								.entity_mut(action)
								.trigger_target(Outcome::Fail);
							return Ok(());
						}
						(false, Some(origin)) => origin,
					};

					if !config.origin_allowed(&origin) {
						world.entity_mut(agent).insert(
							Response::from_status_body(
								StatusCode::Forbidden,
								b"Origin not allowed",
								"text/plain",
							),
						);
						world.entity_mut(action).trigger_target(Outcome::Fail);
						return Ok(());
					}

					// Store validated origin for response phase
					world.entity_mut(agent).insert(ValidatedOrigin(origin));
					world.entity_mut(action).trigger_target(Outcome::Pass);
					Ok(())
				});
			},
		),
	)
}

/// Response-phase CORS middleware that adds CORS headers.
///
/// This middleware:
/// - Reads the [`ValidatedOrigin`] component
/// - Adds CORS headers to the Response component
/// - Triggers [`Outcome::Fail`] if no Response or ValidatedOrigin exists
///
/// Should be paired with [`cors_request`] which validates and stores the origin.
///
/// # Example
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_net::prelude::*;
/// let config = CorsConfig::new(true, vec![]);
/// ExchangeSpawner::new_flow(|| {
///     (InfallibleSequence, children![
///         common_middleware::cors_request(config.clone()),
///         EndpointBuilder::get().with_handler(|| "Hello"),
///         common_middleware::cors_response(config),
///     ])
/// });
/// ```
pub fn cors_response(_config: CorsConfig) -> impl Bundle {
	(
		Name::new("CORS Response Middleware"),
		OnSpawn::observe(
			|ev: On<GetOutcome>, agents: AgentQuery, mut commands: Commands| {
				let action = ev.target();
				let agent = agents.entity(action);

				commands.queue(move |world: &mut World| -> Result {
					// Get the validated origin
					let origin = world
						.entity(agent)
						.get::<ValidatedOrigin>()
						.ok_or_else(|| {
							bevyhow!(
								"No ValidatedOrigin found for CORS response middleware"
							)
						})?
						.0
						.clone();

					// Modify the response to add CORS headers
					let mut entity = world.entity_mut(agent);
					let Some(mut response) = entity.get_mut::<Response>()
					else {
						cross_log!(
							"No Response found for CORS response middleware"
						);
						return Ok(());
					};

					response
						.parts_mut()
						.insert_header("access-control-allow-origin", &origin);

					Ok(())
				});

				commands.entity(action).trigger_target(Outcome::Pass);
			},
		),
	)
}

/// Handle CORS preflight OPTIONS requests.
///
/// This middleware checks if the request method is OPTIONS and if so,
/// inserts a response with appropriate CORS preflight headers and triggers [`Outcome::Pass`].
/// For non-OPTIONS requests, it triggers [`Outcome::Pass`] without inserting a response.
///
/// This combines both request and response phases for preflight handling, storing
/// the validated origin in a [`ValidatedOrigin`] component.
///
/// ## Important: Use with Fallback
///
/// This middleware should be used with [`Fallback`] pattern to prevent endpoints from
/// clobbering the preflight response. The endpoint will only run if preflight didn't
/// insert a response.
///
/// # Example
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_net::prelude::*;
/// let config = CorsConfig::new(true, vec![]);
/// ExchangeSpawner::new_flow(move || {
///     (Fallback, children![
///         // Handles OPTIONS and inserts complete response
///         common_middleware::cors_preflight(config.clone()),
///         // Only runs if not OPTIONS (no response exists yet)
///         EndpointBuilder::any_method().with_handler(|| "Hello"),
///     ])
/// });
/// ```
pub fn cors_preflight(config: CorsConfig) -> impl Bundle {
	(
		Name::new("CORS Preflight Middleware"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      agents: AgentQuery,
			      mut commands: Commands| {
				let action = ev.target();
				let agent = agents.entity(action);
				let config = config.clone();

				commands.queue(move |world: &mut World| -> Result {
					let request = world
						.entity(agent)
						.get::<Request>()
						.ok_or_else(|| {
							bevyhow!(
								"No Request found for CORS preflight middleware"
							)
						})?;

					// Only handle OPTIONS requests
					if *request.method() != HttpMethod::Options {
						world.entity_mut(action).trigger_target(Outcome::Pass);
						return Ok(());
					}

					let origin_header =
						request.get_header("origin").map(|s| s.to_string());

					let origin = match (config.allow_any_origin, origin_header)
					{
						(true, Some(origin)) => origin,
						(true, None) => CorsConfig::ANY_ORIGIN.to_string(),
						(false, None) => {
							world.entity_mut(agent).insert(
								Response::from_status_body(
									StatusCode::MalformedRequest,
									b"Origin header not found",
									"text/plain",
								),
							);
							world
								.entity_mut(action)
								.trigger_target(Outcome::Fail);
							return Ok(());
						}
						(false, Some(origin)) => origin,
					};

					if !config.origin_allowed(&origin) {
						world.entity_mut(agent).insert(
							Response::from_status_body(
								StatusCode::Forbidden,
								b"Origin not allowed",
								"text/plain",
							),
						);
						world.entity_mut(action).trigger_target(Outcome::Fail);
						return Ok(());
					}

					// Store validated origin for potential response middleware
					world
						.entity_mut(agent)
						.insert(ValidatedOrigin(origin.clone()));

					let mut response = Response::ok();
					let parts = response.parts_mut();
					parts.insert_header("access-control-max-age", "60");
					parts.insert_header(
						"access-control-allow-headers",
						"content-type",
					);
					parts.insert_header("access-control-allow-origin", &origin);

					world.entity_mut(agent).insert(response);
					world.entity_mut(action).trigger_target(Outcome::Pass);
					Ok(())
				});
			},
		),
	)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;

	#[sweet::test]
	async fn no_cache_headers_works() {
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![
					EndpointBuilder::get().with_handler(|| "Hello"),
					no_cache_headers(),
				])
			}))
			.oneshot(Request::get("/"))
			.await
			.xtap(|response| {
				response
					.get_header("cache-control")
					.unwrap()
					.xpect_eq("no-cache, no-store, must-revalidate");
				response.get_header("pragma").unwrap().xpect_eq("no-cache");
				response.get_header("expires").unwrap().xpect_eq("0");
			});
	}

	#[sweet::test]
	async fn cors_allows_origin() {
		let config = CorsConfig::new(false, vec!["https://allowed.com"]);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![
					cors_request(config.clone()),
					EndpointBuilder::get().with_handler(|| "Hello"),
					cors_response(config),
				])
			}))
			.oneshot(
				Request::get("/").with_header("origin", "https://allowed.com"),
			)
			.await
			.xtap(|response| {
				response.status().xpect_eq(StatusCode::Ok);
				response
					.get_header("access-control-allow-origin")
					.unwrap()
					.xpect_eq("https://allowed.com");
			});
	}

	#[sweet::test]
	async fn cors_blocks_origin() {
		let config = CorsConfig::new(false, vec![]);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(Sequence, children![
					cors_request(config.clone()),
					EndpointBuilder::get().with_handler(|| "Hello"),
					cors_response(config),
				])
			}))
			.oneshot(
				Request::get("/").with_header("origin", "https://blocked.com"),
			)
			.await
			.status()
			.xpect_eq(StatusCode::Forbidden);
	}

	#[sweet::test]
	async fn cors_allows_any() {
		let config = CorsConfig::new(true, vec![]);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![
					cors_request(config.clone()),
					EndpointBuilder::get().with_handler(|| "Hello"),
					cors_response(config),
				])
			}))
			.oneshot(
				Request::get("/").with_header("origin", "https://anything.com"),
			)
			.await
			.xtap(|response| {
				response.status().xpect_eq(StatusCode::Ok);
				response
					.get_header("access-control-allow-origin")
					.unwrap()
					.xpect_eq("https://anything.com");
			});
	}

	#[sweet::test]
	async fn cors_preflight_works() {
		let config = CorsConfig::new(false, vec!["https://allowed.com"]);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(move || {
				(Fallback, children![
					cors_preflight(config.clone()),
					EndpointBuilder::any_method().with_handler(|| "Hello"),
				])
			}))
			.oneshot(
				Request::options("/")
					.with_header("origin", "https://allowed.com"),
			)
			.await
			.xtap(|response| {
				response.status().xpect_eq(StatusCode::Ok);
				response
					.get_header("access-control-allow-origin")
					.unwrap()
					.xpect_eq("https://allowed.com");
				response
					.get_header("access-control-max-age")
					.unwrap()
					.xpect_eq("60");
			});
	}

	#[sweet::test]
	async fn cors_preflight_non_options_passthrough() {
		let config = CorsConfig::new(true, vec![]);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(move || {
				(InfallibleSequence, children![
					cors_preflight(config.clone()),
					cors_request(config.clone()),
					EndpointBuilder::get().with_handler(|| "Hello"),
					cors_response(config),
				])
			}))
			.oneshot(
				Request::get("/").with_header("origin", "https://example.com"),
			)
			.await
			.xtap(|response| {
				response.status().xpect_eq(StatusCode::Ok);
				response
					.get_header("access-control-allow-origin")
					.unwrap()
					.xpect_eq("https://example.com");
			});
	}

	#[sweet::test]
	async fn multiple_middleware_chain() {
		let config = CorsConfig::new(true, vec![]);
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(move || {
				(InfallibleSequence, children![
					cors_request(config.clone()),
					EndpointBuilder::get().with_handler(|| "Hello"),
					cors_response(config),
					no_cache_headers(),
				])
			}))
			.oneshot(
				Request::get("/").with_header("origin", "https://example.com"),
			)
			.await
			.xtap(|response| {
				response.status().xpect_eq(StatusCode::Ok);
				response
					.get_header("access-control-allow-origin")
					.unwrap()
					.xpect_eq("https://example.com");
				response
					.get_header("cache-control")
					.unwrap()
					.xpect_eq("no-cache, no-store, must-revalidate");
			});
	}
}
