//! Help handler for displaying endpoint documentation.
//!
//! # Kebab-case Parameter Convention
//!
//! This module demonstrates the kebab-case parameter system used throughout beet,
//! the conversion happens automatically:
//! - `ParamMeta::from_field()` converts snake_case field names to kebab-case for display
//! - `MultiMap::parse_reflect()` normalizes kebab-case keys to snake_case before reflection lookup
use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


/// Parameters for configuring the help handler behavior
#[derive(Debug, Clone)]
pub struct HelpHandlerConfig {
	/// Text inserted at the beginning of the formatted output
	pub introduction: String,
	/// The default format to use when rendering help
	pub default_format: HelpFormat,
	/// If true, handler runs when path segments are empty OR --help param is present
	/// If false, handler only runs when --help param is present
	pub match_root: bool,
	/// If true, disable colored output
	pub no_color: bool,
}

impl Default for HelpHandlerConfig {
	fn default() -> Self {
		Self {
			introduction: String::from("Cli Help"),
			default_format: default(),
			match_root: false,
			no_color: false,
		}
	}
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
	Default,
)]
#[reflect(Default)]
pub struct HelpParams {
	#[deref]
	#[reflect(@ParamOptions::desc("Get help"))]
	help: bool,
	#[reflect(@ParamOptions::desc("Help format: cli, http"))]
	help_format: Option<String>,
}


/// Creates a help handler middleware that responds to help parameters.
///
/// This handler checks for `HelpParams` (--help, --help-format) and when found,
/// renders documentation for all endpoints that partially match the current path.
///
/// The handler should be added early in the request flow, typically in a Fallback
/// so it can exit early when help is requested.
///
/// # Arguments
/// * `params` - Configuration parameters for the help handler
///
/// # Example
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # use beet_net::prelude::*;
/// # async {
/// RouterPlugin::world()
///     .spawn(ExchangeSpawner::new_flow(|| {
///         (Fallback, children![
///             help_handler(HelpHandlerConfig {
///                 introduction: String::from("Welcome to my CLI"),
///                 default_format: HelpFormat::Cli,
///                 match_root: false,
///                 no_color: false,
///             }),
///             EndpointBuilder::get()
///                 .with_path("foo")
///                 .with_handler(|| "foo"),
///         ])
///     }))
///     .oneshot_str(Request::get("/?help=true"))
///     .await;
/// # };
/// ```
pub fn help_handler(handler_config: HelpHandlerConfig) -> impl Bundle {
	(
		Name::new("Help Handler"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut route_query: RouteQuery,
			      endpoints_query: Query<&Endpoint>,
			      mut commands: Commands|
			      -> Result {
				let action = ev.target();

				// parse help params
				let req_params = route_query
					.request_meta(action)?
					.params()
					.parse_reflect::<HelpParams>()?;

				// get current path for checking
				let current_path = route_query.path(action)?.clone();
				let is_root = current_path.is_empty();

				// check if help should run:
				// - if match_root is true: run if help param OR path is empty
				// - if match_root is false: run only if help param is present
				let should_run = if handler_config.match_root {
					req_params.help || is_root
				} else {
					req_params.help
				};

				if !should_run {
					// no help requested, pass through
					commands.entity(action).trigger_target(Outcome::Fail);
					return Ok(());
				}



				// get endpoint tree and filter by partial path match
				let tree = route_query.endpoint_tree(action)?;

				// collect all endpoints that match
				let mut matching_endpoints = Vec::new();
				collect_endpoints_from_tree(
					&tree,
					&current_path,
					&mut matching_endpoints,
					&endpoints_query,
				);

				// determine format
				let format = match req_params.help_format.as_deref() {
					Some("http") => HelpFormat::Http,
					Some("cli") => HelpFormat::Cli,
					Some(other) => {
						bevybail!("Unrecognized help-format '{}'", other);
					}
					_ => handler_config.default_format,
				};
				let formatter: Box<dyn EndpointHelpFormatter> = match format {
					HelpFormat::Cli => Box::new(CliFormatter),
					HelpFormat::Http => Box::new(HttpFormatter),
				};


				// render help for all matching endpoints
				let help_text = formatter.format(
					&handler_config,
					&matching_endpoints,
					&current_path,
				);

				let agent = route_query.agents.entity(action);
				commands.entity(agent).insert(
					Response::new(default(), help_text.into())
						.with_content_type("text/plain"),
				);

				// pass to exit fallback early
				commands.entity(action).trigger_target(Outcome::Pass);

				Ok(())
			},
		),
	)
}


// recursively collect all endpoints from the tree
fn collect_endpoints_from_tree(
	node: &EndpointTree,
	current_path: &Vec<String>,
	endpoints: &mut Vec<Endpoint>,
	endpoints_query: &Query<&Endpoint>,
) {
	let node_depth = node.pattern.iter().count();
	let current_depth = current_path.len();

	// determine if we should include this node and/or recurse
	if current_path.is_empty() {
		// show all endpoints when no filter specified
		if let Some(entity) = node.endpoint {
			if let Ok(endpoint) = endpoints_query.get(entity) {
				endpoints.push(endpoint.clone());
			}
		}
		// always recurse when no filter
		for child in &node.children {
			collect_endpoints_from_tree(
				child,
				current_path,
				endpoints,
				endpoints_query,
			);
		}
	} else if node_depth <= current_depth {
		// node is at or above current depth - check if it's on the path
		match node.pattern.parse_path(current_path) {
			Ok(path_match) => {
				// pattern matches current path
				if path_match.exact_match() {
					// exact match - show this endpoint and all children
					if let Some(entity) = node.endpoint {
						if let Ok(endpoint) = endpoints_query.get(entity) {
							endpoints.push(endpoint.clone());
						}
					}
				}
				// recurse to children since we're on the right path
				for child in &node.children {
					collect_endpoints_from_tree(
						child,
						current_path,
						endpoints,
						endpoints_query,
					);
				}
			}
			Err(_) => {
				// pattern doesn't match - don't include or recurse
			}
		}
	}
	// if node_depth > current_depth, we've gone too deep, don't include
}

/// Trait for formatting endpoint help documentation
pub trait EndpointHelpFormatter {
	/// Format the complete help output for multiple endpoints
	fn format(
		&self,
		params: &HelpHandlerConfig,
		endpoints: &[Endpoint],
		path: &Vec<String>,
	) -> String {
		let _enabled = paint_ext::SetPaintEnabledTemp::new(!params.no_color);

		if endpoints.is_empty() {
			return self.format_none_found(path);
		}

		let mut output = String::new();
		output.push_str("\n");
		output.push_str(&params.introduction);
		output.push_str("\n\n");
		output.push_str(&self.format_header());
		output.push_str("\n\n");

		for endpoint in endpoints {
			output.push_str(&self.format_endpoint(endpoint));
			output.push_str("\n");
		}

		output
	}

	fn format_none_found(&self, path: &Vec<String>) -> String;

	/// Format the header section
	fn format_header(&self) -> String;

	/// Format a single endpoint
	fn format_endpoint(&self, endpoint: &Endpoint) -> String;

	/// Format the path
	fn format_path(&self, path: &PathPattern) -> String;

	/// Format the parameters/flags
	fn format_params(&self, endpoint: &Endpoint) -> String;

	/// Format cache strategy (if applicable)
	fn format_cache_strategy(&self, cache: &CacheStrategy) -> String {
		format!("Cache: {:?}", cache)
	}

	/// Format content type (if applicable)
	fn format_content_type(&self, content_type: &ContentType) -> String {
		format!("Content-Type: {:?}", content_type)
	}
}

/// CLI-style formatter with colored output
struct CliFormatter;

impl EndpointHelpFormatter for CliFormatter {
	fn format_header(&self) -> String {
		format!("\n{}", paint_ext::bold("Available commands:"))
	}

	fn format_endpoint(&self, endpoint: &Endpoint) -> String {
		let mut output = String::new();

		// command path (CLI-style: space-separated, no slashes)
		let path_str = self.format_path(endpoint.path());
		output.push_str(&format!("  {}", paint_ext::green(&path_str)));

		// description
		if let Some(desc) = endpoint.description() {
			output.push_str(&format!("\n    {}", desc));
		}

		// params
		output.push_str(&self.format_params(endpoint));

		output
	}

	fn format_path(&self, path: &PathPattern) -> String {
		path.iter()
			.map(|seg| {
				if seg.is_static() {
					seg.to_string()
				} else if seg.is_greedy() {
					format!("[*{}]", seg.name())
				} else {
					format!("[{}]", seg.name())
				}
			})
			.collect::<Vec<_>>()
			.join(" ")
	}



	fn format_params(&self, endpoint: &Endpoint) -> String {
		if endpoint.params().is_empty() {
			return String::new();
		}

		let mut output = String::new();
		output.push_str(&format!("\n    {}", paint_ext::dimmed("Flags:")));

		for param in endpoint.params().iter() {
			output.push_str("\n      ");

			// name with short flag if present
			let mut param_display = format!("--{}", param.name());
			if let Some(short) = param.short() {
				param_display.push_str(&format!(", -{}", short));
			}
			output
				.push_str(&paint_ext::yellow(format!("{:20}", param_display)));

			// value type
			match param.value() {
				ParamValue::Flag => {
					output.push_str(&paint_ext::dimmed("(flag)     "))
				}
				ParamValue::Single => {
					output.push_str(&paint_ext::dimmed("(value)    "))
				}
				ParamValue::Multiple => {
					output.push_str(&paint_ext::dimmed("(multiple) "))
				}
			}

			// required/optional
			if param.is_required() {
				output.push_str(&paint_ext::red("required"));
			} else {
				output.push_str(&paint_ext::green("optional"));
			}

			// description
			if let Some(desc) = param.description() {
				output.push_str(&format!(" - {}", desc));
			}
		}

		output
	}

	fn format_none_found(&self, path: &Vec<String>) -> String {
		let path_str = if path.is_empty() {
			"<empty>".to_string()
		} else {
			path.join(" ")
		};
		paint_ext::red_bold(format!(
			"No matching endpoints found for path: {}",
			path_str
		))
		.to_string()
	}
}

/// HTTP-style formatter with colored output
struct HttpFormatter;

impl EndpointHelpFormatter for HttpFormatter {
	fn format_header(&self) -> String {
		paint_ext::bold("Available endpoints:").to_string()
	}

	fn format_endpoint(&self, endpoint: &Endpoint) -> String {
		let mut output = String::new();

		// method and path
		let method = endpoint
			.method()
			.map(|m| format!("{:?}", m).to_uppercase())
			.unwrap_or_else(|| "ANY".to_string());
		let path = endpoint.path().annotated_route_path();

		output.push_str(&format!(
			"  {} {}",
			paint_ext::cyan(&method),
			paint_ext::green(&path.to_string())
		));

		// description
		if let Some(desc) = endpoint.description() {
			output.push_str(&format!("\n    {}", desc));
		}

		// query params
		output.push_str(&self.format_params(endpoint));

		// content type
		if let Some(content_type) = endpoint.content_type() {
			output.push_str(&format!(
				"\n    {}",
				paint_ext::dimmed(&self.format_content_type(&content_type))
			));
		}

		// cache strategy
		if let Some(cache) = endpoint.cache_strategy() {
			output.push_str(&format!(
				"\n    {}",
				paint_ext::dimmed(&self.format_cache_strategy(&cache))
			));
		}

		output
	}

	fn format_path(&self, path: &PathPattern) -> String {
		path.annotated_route_path().to_string()
	}



	fn format_params(&self, endpoint: &Endpoint) -> String {
		if endpoint.params().is_empty() {
			return String::new();
		}

		let mut output = String::new();
		output.push_str(&format!(
			"\n    {}",
			paint_ext::bold("Query Parameters:")
		));

		for param in endpoint.params().iter() {
			output.push_str(&format!(
				"\n      {}",
				paint_ext::yellow(param.name())
			));
			if param.is_required() {
				output.push_str(&paint_ext::red(" (required)"));
			} else {
				output.push_str(&paint_ext::green(" (optional)"));
			}
			output.push_str(&format!(
				" - {}",
				paint_ext::dimmed(&param.value().to_string())
			));
			if let Some(desc) = param.description() {
				output.push_str(&format!(": {}", desc));
			}
		}

		output
	}

	fn format_none_found(&self, path: &Vec<String>) -> String {
		let path_str = if path.is_empty() {
			default()
		} else {
			path.join("/")
		};
		paint_ext::red_bold(format!(
			"No matching endpoints found for path: /{}",
			path_str
		))
		.to_string()
	}
}

/// Format for rendering endpoint help
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum HelpFormat {
	#[default]
	/// CLI format with commands and flags
	Cli,
	/// HTTP format with methods and query params
	Http,
}

#[cfg(test)]
mod test {
	use super::*;

	#[sweet::test]
	async fn help_shows_matching_endpoints() {
		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig::default()),
				EndpointBuilder::get()
					.with_path("foo")
					.with_description("The foo command")
					.with_handler(|| "foo"),
				EndpointBuilder::get()
					.with_path("bar")
					.with_description("The bar command")
					.with_handler(|| "bar"),
			])
		}));

		let response = entity.oneshot_str(Request::get("/?help=true")).await;

		response.clone().xpect_contains("foo");
		response.clone().xpect_contains("The foo command");
		response.clone().xpect_contains("bar");
		response.xpect_contains("The bar command");
	}

	#[sweet::test]
	async fn help_format_http() {
		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig::default()),
				EndpointBuilder::post()
					.with_path("api/users")
					.with_description("Create user")
					.with_handler(|| "create"),
			])
		}));

		let response = entity
			.oneshot_str(Request::get("/?help=true&help-format=http"))
			.await;

		response.clone().xpect_contains("POST");
		response.clone().xpect_contains("api/users");
		response.xpect_contains("Create user");
	}

	#[sweet::test]
	async fn help_with_params() {
		#[derive(Reflect)]
		struct TestParams {
			#[reflect(@ParamOptions::desc("Enable verbose output"))]
			verbose: bool,
			#[reflect(@ParamOptions::desc_and_short("Output format", 'f'))]
			format: Option<String>,
		}

		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig::default()),
				EndpointBuilder::get()
					.with_path("test")
					.with_params::<TestParams>()
					.with_description("Test command")
					.with_handler(|| "test"),
			])
		}));

		let response = entity.oneshot_str(Request::get("/?help=true")).await;

		response.clone().xpect_contains("verbose");
		response.clone().xpect_contains("Enable verbose output");
		response.clone().xpect_contains("format");
		response.xpect_contains(", -f");
	}

	#[sweet::test]
	async fn no_help_passes_through() {
		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig::default()),
				EndpointBuilder::get()
					.with_path("foo")
					.with_handler(|| "foo response"),
			])
		}));

		let response = entity.oneshot_str(Request::get("/foo")).await;

		response.xpect_eq("foo response");
	}

	#[sweet::test]
	async fn kebab_case_params_work() {
		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig::default()),
				EndpointBuilder::get()
					.with_path("test")
					.with_handler(|| "test"),
			])
		}));

		// test both kebab-case and underscore variants
		let response1 = entity
			.oneshot_str(Request::get("/?help=true&help-format=http"))
			.await;
		response1.clone().xpect_contains("GET");

		let response2 = entity
			.oneshot_str(Request::get("/?help=true&help_format=http"))
			.await;
		response2.xpect_contains("GET");
	}

	#[sweet::test]
	async fn full_kebab_case_flow_integration() {
		// demonstrates the complete kebab-case parameter system:
		// 1. struct fields use snake_case
		// 2. ParamMeta displays them as kebab-case
		// 3. query params accept both formats
		// 4. MultiMap normalizes to snake_case for reflection

		#[derive(Reflect)]
		struct TestParams {
			#[reflect(@ParamOptions::desc("Maximum retry attempts"))]
			max_retry_count: u32,
			#[reflect(@ParamOptions::desc("Enable verbose logging"))]
			enable_verbose_mode: bool,
		}

		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig::default()),
				EndpointBuilder::get()
					.with_path("deploy")
					.with_params::<TestParams>()
					.with_description("Deploy application")
					.with_handler(|| "deployed"),
			])
		}));

		// CLI help shows kebab-case params
		let help_response =
			entity.oneshot_str(Request::get("/?help=true")).await;

		help_response.clone().xpect_contains("--max-retry-count");
		help_response
			.clone()
			.xpect_contains("--enable-verbose-mode");
		help_response
			.clone()
			.xpect_contains("Maximum retry attempts");
		help_response.xpect_contains("Enable verbose logging");

		// HTTP help also shows kebab-case
		let http_help = entity
			.oneshot_str(Request::get("/?help=true&help-format=http"))
			.await;

		http_help.clone().xpect_contains("max-retry-count");
		http_help.xpect_contains("enable-verbose-mode");
	}
}
