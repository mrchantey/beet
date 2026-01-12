use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

/// Predicate that checks for `--help` param on the request.
///
/// If the `--help` param is not present, this predicate will pass through immediately.
/// If present, it renders and returns formatted help documentation including:
/// - Usage line with method and path
/// - Endpoint details (method, path, content type, cache strategy)
/// - Path segment breakdown (static vs dynamic segments)
/// - Query parameters with descriptions and requirements
/// - Available direct child subcommands
///
/// The help is returned as a response and the predicate fails to prevent
/// further processing of the request.
///
/// By default, renders in CLI format. Use `--help-format=http` to render in HTTP format.
///
/// # Example
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// EndpointBuilder::get()
///     .with_path("api/users")
///     .with_description("List all users")
///     .with_handler(|| "list users");
/// ```
pub fn endpoint_help_predicate() -> impl Bundle {
	(
		Name::new("Endpoint Help"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 ancestors: Query<&ChildOf>,
			 children: Query<&Children>,
			 mut commands: Commands,
			 route_query: RouteQuery,
			 endpoints_query: Query<&Endpoint>|
			 -> Result {
				let action = ev.target();
				if !route_query.request_meta(action)?.has_param("help") {
					// no help requested, pass through
					commands.entity(action).trigger_target(Outcome::Pass);
					return Ok(());
				}

				// Determine format from --help-format parameter
				let format = route_query
					.request_meta(action)?
					.get_param("help-format")
					.and_then(|f| match f.as_str() {
						"http" => Some(HelpFormat::Http),
						"cli" => Some(HelpFormat::Cli),
						_ => None,
					})
					.unwrap_or_default();

				// Find the closest endpoint ancestor (if any)
				let endpoint_entity = ancestors
					.iter_ancestors_inclusive(action)
					.find(|e| endpoints_query.contains(*e));

				let endpoint =
					endpoint_entity.and_then(|e| endpoints_query.get(e).ok());

				// Find sibling endpoints as subcommands, but only for root/empty path
				let mut subcommands = Vec::new();
				if let Some(endpoint_entity) = endpoint_entity {
					// Only show siblings if this is the root endpoint (empty path)
					let is_root = endpoint
						.map(|ep| ep.path().is_empty())
						.unwrap_or(false);

					if is_root {
						// Get the parent of the endpoint to find siblings
						if let Some(parent) =
							ancestors.iter_ancestors(endpoint_entity).next()
						{
							// Get all children of the parent (siblings of current endpoint)
							if let Ok(parent_children) = children.get(parent) {
								for sibling in parent_children.iter() {
									// Skip the current endpoint
									if sibling == endpoint_entity {
										continue;
									}
									// Check if this sibling has an Endpoint component
									if let Ok(sibling_endpoint) =
										endpoints_query.get(sibling)
									{
										// Skip empty path endpoints in subcommand list
										if !sibling_endpoint.path().is_empty() {
											subcommands.push(sibling_endpoint);
										}
									}
								}
							}
						}
					}
				}



				let help = EndpointHelp {
					endpoint,
					subcommands,
					format,
				};

				let help_text = help.render();

				commands
					.entity(route_query.requests.entity(action))
					.insert(Response::new(default(), help_text.into()));

				// mark as failed to stop further processing
				commands.entity(action).trigger_target(Outcome::Fail);

				Ok(())
			},
		),
	)
}

/// Format for rendering endpoint help
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpFormat {
	/// CLI format with commands and flags
	Cli,
	/// HTTP format with methods and query params
	Http,
}

impl Default for HelpFormat {
	fn default() -> Self { Self::Cli }
}

/// Helper for rendering endpoint help documentation
pub struct EndpointHelp<'a> {
	/// The current endpoint if any
	pub endpoint: Option<&'a Endpoint>,
	/// Direct child endpoints (subcommands/subpaths)
	pub subcommands: Vec<&'a Endpoint>,
	/// Format to render in
	pub format: HelpFormat,
}

impl<'a> EndpointHelp<'a> {
	/// Render the help documentation
	pub fn render(&self) -> String {
		match self.format {
			HelpFormat::Cli => self.render_cli(),
			HelpFormat::Http => self.render_http(),
		}
	}

	/// Render help in CLI format
	pub fn render_cli(&self) -> String {
		let mut output = String::new();

		// Usage line
		if let Some(endpoint) = self.endpoint {
			output.push_str(&paint_ext::bold("Usage: "));

			// Binary name from args[0], fallback to package name
			let binary_name = std::env::args()
				.next()
				.and_then(|path| {
					std::path::Path::new(&path)
						.file_name()
						.and_then(|n| n.to_str())
						.map(|s| s.to_string())
				})
				.unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string());
			output.push_str(&binary_name);

			// Path as positional arguments
			let path_str = self.format_path_cli(endpoint.path());
			if !path_str.is_empty() {
				output.push(' ');
				output.push_str(&paint_ext::green(path_str));
			}

			output.push_str("\n\n");

			// Description if present
			if let Some(desc) = endpoint.description() {
				output.push_str(desc);
				output.push_str("\n\n");
			}

			// Command details (only show if we have a non-empty path)
			let path_str = self.format_path_cli(endpoint.path());
			if !path_str.is_empty() {
				output.push_str(&paint_ext::bold("Command:\n"));
				output.push_str("  ");

				// Path
				output.push_str(&paint_ext::green(path_str));

				// Content type
				if let Some(content_type) = endpoint.content_type() {
					output.push_str(&format!(
						" {}",
						paint_ext::dimmed(format!("[{:?}]", content_type))
					));
				}

				// Cache strategy
				if let Some(cache) = endpoint.cache_strategy() {
					output.push_str(&format!(
						" {}",
						paint_ext::dimmed(format!("[{:?}]", cache))
					));
				}

				output.push_str("\n");
			}

			// Path segments breakdown
			if !endpoint.path().is_empty() {
				let has_dynamic =
					endpoint.path().iter().any(|seg| !seg.is_static());
				if has_dynamic {
					output.push_str(&format!(
						"    {}\n",
						paint_ext::dimmed("Arguments:")
					));
					for segment in endpoint.path().iter() {
						if !segment.is_static() {
							output.push_str(&format!(
								"      {} - ",
								paint_ext::yellow(
									self.format_segment_cli(segment)
								)
							));
							if segment.is_greedy() {
								output.push_str(&paint_ext::dimmed(
									"(captures one or more)",
								));
							} else {
								output
									.push_str(&paint_ext::dimmed("(required)"));
							}
							output.push_str("\n");
						}
					}
				}
			}

			// Parameters
			if !endpoint.params().is_empty() {
				output.push_str(&format!(
					"    {}\n",
					paint_ext::dimmed("Flags:")
				));
				for param in endpoint.params().iter() {
					output.push_str("      ");

					// Name with short flag if present
					let mut param_display = format!("--{}", param.name());
					if let Some(short) = param.short() {
						param_display.push_str(&format!(", -{}", short));
					}
					output.push_str(&paint_ext::yellow(format!(
						"{:20}",
						param_display
					)));

					// Value type
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

					// Required/Optional
					if param.is_required() {
						output.push_str(&paint_ext::red("required"));
					} else {
						output.push_str(&paint_ext::green("optional"));
					}

					// Description
					if let Some(desc) = param.description() {
						output.push_str(&format!(" - {}", desc));
					}

					output.push_str("\n");
				}
			}
			output.push_str("\n");
		}

		// Subcommands
		if !self.subcommands.is_empty() {
			output.push_str(&paint_ext::bold("Available subcommands:\n"));
			for endpoint in &self.subcommands {
				output.push_str("  ");

				// Path
				output.push_str(&paint_ext::green(
					self.format_path_cli(endpoint.path()),
				));

				// Description if present
				if let Some(desc) = endpoint.description() {
					output.push_str(&format!(" - {}", paint_ext::dimmed(desc)));
				}

				output.push_str("\n");
			}
		}

		output
	}

	/// Render help in HTTP format
	pub fn render_http(&self) -> String {
		let mut output = String::new();

		// Usage line
		if let Some(endpoint) = self.endpoint {
			output.push_str(&paint_ext::bold("Usage: "));

			// Method
			if let Some(method) = endpoint.method() {
				output.push_str(&paint_ext::cyan(format!("{} ", method)));
			}

			// Path
			output.push_str(&paint_ext::green(
				endpoint.path().annotated_route_path().to_string(),
			));

			// Query params
			if !endpoint.params().is_empty() {
				output.push('?');
				let param_strs: Vec<String> = endpoint
					.params()
					.iter()
					.map(|p| self.format_param_http(p))
					.collect();
				output.push_str(&param_strs.join("&"));
			}

			output.push_str("\n\n");

			// Description if present
			if let Some(desc) = endpoint.description() {
				output.push_str(desc);
				output.push_str("\n\n");
			}

			// Endpoint details
			output.push_str(&paint_ext::bold("Endpoint:\n"));
			output.push_str("  ");

			// Method
			if let Some(method) = endpoint.method() {
				output.push_str(&paint_ext::cyan(format!(
					"{:6}",
					format!("{}", method)
				)));
			} else {
				output.push_str(&paint_ext::dimmed("ANY   "));
			}
			output.push_str(" ");

			// Path
			output.push_str(&paint_ext::green(
				endpoint.path().annotated_route_path().to_string(),
			));

			// Content type
			if let Some(content_type) = endpoint.content_type() {
				output.push_str(&format!(
					" {}",
					paint_ext::dimmed(format!("[{:?}]", content_type))
				));
			}

			// Cache strategy
			if let Some(cache) = endpoint.cache_strategy() {
				output.push_str(&format!(
					" {}",
					paint_ext::dimmed(format!("[{:?}]", cache))
				));
			}

			output.push_str("\n");

			// Path segments breakdown
			if !endpoint.path().is_empty() {
				let has_dynamic =
					endpoint.path().iter().any(|seg| !seg.is_static());
				if has_dynamic {
					output.push_str(&format!(
						"    {}\n",
						paint_ext::dimmed("Path segments:")
					));
					for segment in endpoint.path().iter() {
						if !segment.is_static() {
							output.push_str(&format!(
								"      {} - ",
								paint_ext::yellow(
									segment.to_string_annotated()
								)
							));
							if segment.is_greedy() {
								output.push_str(&paint_ext::dimmed(
									"(captures one or more)",
								));
							} else {
								output
									.push_str(&paint_ext::dimmed("(dynamic)"));
							}
							output.push_str("\n");
						}
					}
				}
			}

			// Parameters
			if !endpoint.params().is_empty() {
				output.push_str(&format!(
					"    {}\n",
					paint_ext::dimmed("Query parameters:")
				));
				for param in endpoint.params().iter() {
					output.push_str("      ");

					// Name
					output.push_str(&paint_ext::yellow(format!(
						"{:20}",
						self.format_param_http(param)
					)));

					// Value type
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

					// Required/Optional
					if param.is_required() {
						output.push_str(&paint_ext::red("required"));
					} else {
						output.push_str(&paint_ext::green("optional"));
					}

					// Description
					if let Some(desc) = param.description() {
						output.push_str(&format!(" - {}", desc));
					}

					output.push_str("\n");
				}
			}
			output.push_str("\n");
		}

		// Subpaths
		if !self.subcommands.is_empty() {
			output.push_str(&paint_ext::bold("Available endpoints:\n"));
			for endpoint in &self.subcommands {
				output.push_str("  ");

				// Method
				if let Some(method) = endpoint.method() {
					output.push_str(&paint_ext::cyan(format!("{:6} ", method)));
				} else {
					output.push_str(&paint_ext::dimmed("ANY    "));
				}

				// Path
				output.push_str(&paint_ext::green(
					endpoint.path().annotated_route_path().to_string(),
				));

				// Description if present
				if let Some(desc) = endpoint.description() {
					output.push_str(&format!(" - {}", paint_ext::dimmed(desc)));
				}

				output.push_str("\n");
			}
		}

		output
	}

	/// Format path for CLI rendering
	fn format_path_cli(&self, path: &PathPattern) -> String {
		path.iter()
			.map(|seg| self.format_segment_cli(seg))
			.collect::<Vec<_>>()
			.join(" ")
	}

	/// Format a single segment for CLI rendering
	fn format_segment_cli(&self, segment: &PathPatternSegment) -> String {
		if segment.is_static() {
			segment.to_string()
		} else if segment.is_greedy() {
			format!("[*{}]", segment.name())
		} else {
			format!("[{}]", segment.name())
		}
	}

	/// Format param for HTTP rendering
	fn format_param_http(&self, param: &ParamMeta) -> String {
		match param.value() {
			ParamValue::Flag => param.name().to_string(),
			_ => format!("{}=<value>", param.name()),
		}
	}
}

// Integration tests in endpoint_help.rs action file verify the full behavior


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[sweet::test]
	async fn simple() {
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				EndpointBuilder::get()
					.with_path("foobar")
					.with_description("A simple endpoint for testing")
			}))
			.oneshot_str(Request::get("/foobar").with_query_param("help", ""))
			.await
			.xpect_snapshot();
	}

	#[sweet::test]
	async fn with_params_and_dynamic_path() {
		#[derive(Reflect)]
		struct TestParams {
			#[reflect(@ParamOptions::desc_and_short("Enable verbose output", 'v'))]
			verbose: bool,
			#[reflect(@ParamOptions::desc("Output format"))]
			format: Option<String>,
			#[reflect(@ParamOptions::desc("Tags to filter"))]
			tags: Vec<String>,
		}

		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				EndpointBuilder::get()
					.with_path("api/:version/items/*path")
					.with_params::<TestParams>()
					.with_description("Fetch items with dynamic versioning")
			}))
			.oneshot_str(
				Request::get("/api/v1/items/foo/bar")
					.with_query_param("help", ""),
			)
			.await
			.xpect_snapshot();
	}

	#[sweet::test]
	async fn http_format() {
		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				EndpointBuilder::get()
					.with_path("foobar")
					.with_description("A simple endpoint for testing")
			}))
			.oneshot_str(
				Request::get("/foobar")
					.with_query_param("help", "")
					.with_query_param("help-format", "http"),
			)
			.await
			.xpect_snapshot();
	}

	#[sweet::test]
	async fn root_shows_sibling_subcommands() {
		use beet_flow::prelude::*;

		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![
					EndpointBuilder::get()
						.with_path("")
						.with_description("Root endpoint")
						.with_handler(|| "root"),
					EndpointBuilder::get()
						.with_path("users")
						.with_description("List all users")
						.with_handler(|| "list users"),
					EndpointBuilder::get()
						.with_path("posts")
						.with_description("List all posts")
						.with_handler(|| "list posts"),
				])
			}))
			.oneshot_str(Request::get("/").with_query_param("help", ""))
			.await
			.xpect_snapshot();
	}

	#[sweet::test]
	async fn no_subcommands_for_leaf_endpoint() {
		use beet_flow::prelude::*;

		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![(
					PathPartial::new("api"),
					Sequence,
					children![
						EndpointBuilder::get()
							.with_path("teapot")
							.with_description("I'm a teapot")
							.with_handler(|| "teapot"),
						EndpointBuilder::get()
							.with_path("users")
							.with_description("List all users")
							.with_handler(|| "list users"),
					]
				),])
			}))
			.oneshot_str(
				Request::get("/api/teapot").with_query_param("help", ""),
			)
			.await
			.xpect_snapshot();
	}
}
