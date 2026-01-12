use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

/// Simple paint utilities for help output
mod paint {
	use std::fmt::Display;

	pub fn bold(val: impl Display) -> String {
		format!("\x1b[1m{}\x1b[0m", val)
	}

	pub fn dimmed(val: impl Display) -> String {
		format!("\x1b[2m{}\x1b[0m", val)
	}

	pub fn green(val: impl Display) -> String {
		format!("\x1b[32m{}\x1b[0m", val)
	}

	pub fn yellow(val: impl Display) -> String {
		format!("\x1b[33m{}\x1b[0m", val)
	}

	pub fn cyan(val: impl Display) -> String {
		format!("\x1b[36m{}\x1b[0m", val)
	}

	pub fn red(val: impl Display) -> String {
		format!("\x1b[31m{}\x1b[0m", val)
	}
}


struct EndpointHelp {
	prefix: String,
	/// endpoints for this point in the tree,
	/// from start to end
	endpoints: Vec<Endpoint>,
	/// Sibling endpoints that can be navigated to
	sibling_endpoints: Vec<Endpoint>,
}

impl EndpointHelp {
	fn render(&self) -> String {
		let mut output = String::new();

		// Header
		output.push_str(&paint::bold(format!("Usage: {}", self.prefix)));

		// If we have endpoints, show them
		if !self.endpoints.is_empty() {
			output.push_str(" ");

			// Show method if consistent across endpoints
			let methods: Vec<_> =
				self.endpoints.iter().filter_map(|e| e.method()).collect();
			if methods.len() == self.endpoints.len()
				&& methods.iter().all(|m| *m == methods[0])
			{
				output.push_str(&paint::cyan(format!("[{}]", methods[0])));
				output.push_str(" ");
			}

			// Show path
			if let Some(first) = self.endpoints.first() {
				output.push_str(&paint::green(
					first.path().annotated_route_path().to_string(),
				));
			}
		}

		output.push_str("\n\n");

		// Endpoints section
		if !self.endpoints.is_empty() {
			output.push_str(&paint::bold("Endpoints:\n"));

			for endpoint in &self.endpoints {
				output.push_str("  ");

				// Method
				if let Some(method) = endpoint.method() {
					output.push_str(&paint::cyan(format!(
						"{:6}",
						format!("{}", method)
					)));
				} else {
					output.push_str(&paint::dimmed("ANY   "));
				}
				output.push_str(" ");

				// Path
				output.push_str(&paint::green(
					endpoint.path().annotated_route_path().to_string(),
				));

				// Content type
				if let Some(content_type) = endpoint.content_type() {
					output.push_str(&format!(
						" {}",
						paint::dimmed(format!("[{:?}]", content_type))
					));
				}

				// Cache strategy
				if let Some(cache) = endpoint.cache_strategy() {
					output.push_str(&format!(
						" {}",
						paint::dimmed(format!("[{:?}]", cache))
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
							paint::dimmed("Path segments:")
						));
						for segment in endpoint.path().iter() {
							if !segment.is_static() {
								output.push_str(&format!(
									"      {} - ",
									paint::yellow(
										segment.to_string_annotated()
									)
								));
								if segment.is_greedy() {
									output.push_str(&paint::dimmed(
										"(captures one or more)",
									));
								} else {
									output
										.push_str(&paint::dimmed("(dynamic)"));
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
						paint::dimmed("Parameters:")
					));
					for param in endpoint.params().iter() {
						output.push_str("      ");

						// Name with short flag if present
						let mut param_display = format!("--{}", param.name());
						if let Some(short) = param.short() {
							param_display.push_str(&format!(", -{}", short));
						}
						output.push_str(&paint::yellow(format!(
							"{:20}",
							param_display
						)));

						// Value type
						match param.value() {
							ParamValue::Flag => {
								output.push_str(&paint::dimmed("(flag)     "))
							}
							ParamValue::Single => {
								output.push_str(&paint::dimmed("(value)    "))
							}
							ParamValue::Multiple => {
								output.push_str(&paint::dimmed("(multiple) "))
							}
						}

						// Required/Optional
						if param.is_required() {
							output.push_str(&paint::red("required"));
						} else {
							output.push_str(&paint::green("optional"));
						}

						// Description
						if let Some(desc) = param.description() {
							output.push_str(&format!(" - {}", desc));
						}

						output.push_str("\n");
					}
				}
			}
			output.push_str("\n");
		}

		// Sibling endpoints / subcommands
		if !self.sibling_endpoints.is_empty() {
			output.push_str(&paint::bold("Available subcommands:\n"));
			for endpoint in &self.sibling_endpoints {
				output.push_str("  ");

				// Method
				if let Some(method) = endpoint.method() {
					output.push_str(&paint::cyan(format!("{:6} ", method)));
				} else {
					output.push_str(&paint::dimmed("ANY    "));
				}

				// Path
				output.push_str(&paint::green(
					endpoint.path().annotated_route_path().to_string(),
				));
				output.push_str("\n");
			}
		}

		output
	}
}

/// Predicate that checks for `--help` param on the request.
///
/// If the `--help` param is not present, this predicate will pass through immediately.
/// If present, it renders and returns formatted help documentation including:
/// - Usage line with method and path
/// - Endpoint details (method, path, content type, cache strategy)
/// - Path segment breakdown (static vs dynamic segments)
/// - Query parameters with descriptions and requirements
/// - Available sibling subcommands
///
/// The help is returned as a response and the predicate fails to prevent
/// further processing of the request.
///
/// # Example
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// EndpointBuilder::get()
///     .with_path("api/users")
///     .with_predicate(endpoint_help_predicate("myapp"))
///     .with_handler(|| "list users");
/// ```
pub fn endpoint_help_predicate(prefix: &str) -> impl Bundle {
	let prefix = prefix.to_string();
	(
		Name::new("Endpoint Help"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
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

				// Collect all endpoints in the ancestry chain
				let mut endpoints = ancestors
					.iter_ancestors_inclusive(action)
					.filter_map(|entity| endpoints_query.get(entity).ok())
					.cloned()
					.collect::<Vec<_>>();
				// start at first one
				endpoints.reverse();

				// Collect sibling endpoints (potential subcommands)
				// First find the endpoint entity in our ancestry
				let mut sibling_endpoints = Vec::new();
				if let Some(endpoint_entity) = ancestors
					.iter_ancestors_inclusive(action)
					.find(|e| endpoints_query.contains(*e))
				{
					// Now get the parent of this endpoint entity
					if let Ok(parent_child_of) = ancestors.get(endpoint_entity)
					{
						let parent_entity = parent_child_of.parent();
						// Get siblings of the endpoint entity
						if let Ok(siblings) = children.get(parent_entity) {
							for sibling in siblings.iter() {
								// Skip self and check if sibling has Endpoint
								if sibling != endpoint_entity {
									if let Ok(endpoint) =
										endpoints_query.get(sibling)
									{
										sibling_endpoints
											.push(endpoint.clone());
									}
								}
							}
						}
					}
				}

				let help = EndpointHelp {
					prefix: prefix.clone(),
					endpoints,
					sibling_endpoints,
				};

				let help_text = help.render();
				help_text.clone().xprint_display();

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
					.with_predicate(endpoint_help_predicate("bangboom"))
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
					.with_predicate(endpoint_help_predicate("mycli"))
			}))
			.oneshot_str(
				Request::get("/api/v1/items/foo/bar")
					.with_query_param("help", ""),
			)
			.await
			.xpect_snapshot();
	}

	#[sweet::test]
	async fn with_children() {
		use beet_flow::prelude::*;

		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![
					(PathPartial::new("api"), Sequence, children![
						EndpointBuilder::get()
							.with_path("users")
							.with_predicate(endpoint_help_predicate("app"))
							.with_handler(|| "list users"),
						EndpointBuilder::post()
							.with_path("items/:id")
							.with_handler(|| "update item"),
					]),
					EndpointBuilder::get()
						.with_path("posts/*slug")
						.with_handler(|| "get post"),
				])
			}))
			.oneshot_str(
				Request::get("/api/users").with_query_param("help", ""),
			)
			.await
			.xpect_snapshot();
	}
}
