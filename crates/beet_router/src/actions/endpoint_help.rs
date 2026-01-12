use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


struct EndpointHelp<'a> {
	prefix: String,
	/// endpoints for this point in the tree,
	/// from start to end
	ancestors: Vec<&'a Endpoint>,
	descendents: Vec<&'a Endpoint>,
}

impl EndpointHelp<'_> {
	fn render(&self) -> String {
		let mut output = String::new();
		output.push_str(&format!("{}\n", self.prefix));
		for endpoint in &self.ancestors {
			output.push_str(&format!("{:?}\n", endpoint));
		}
		for endpoint in &self.descendents {
			output.push_str(&format!("{:?}\n", endpoint));
		}
		output
	}
}

/// Predicate that checks for `--help` flag on the request.
/// If none present this predicate will succeed immediately,
/// otherwise returns help for the endpoint and its ancestors.
pub fn endpoint_help_predicate(prefix: &str) -> impl Bundle {
	let prefix = prefix.to_string();
	(
		Name::new("Endpoint Help"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut commands: Commands,
			      ancestor_query: Query<&ChildOf>,
			      children_query: Query<&Children>,
			      route_query: RouteQuery,
			      endpoint_query: Query<&Endpoint>,
			      path_partial_query: Query<(Entity, &PathPartial)>|
			      -> Result {
				let action = ev.target();
				if !route_query.request_meta(action)?.has_param("help") {
					// no help requested, pass through
					commands.entity(action).trigger_target(Outcome::Pass);
					return Ok(());
				}
				let mut ancestors = ancestor_query
					.iter_ancestors_inclusive(action)
					.filter_map(|entity| endpoint_query.get(entity).ok())
					// .cloned()
					.collect::<Vec<_>>();
				// start at first one
				ancestors.reverse();

				// walk up to the first PathPartial and collect all
				// endpoints descendent from it.
				let descendents =
					if let Some((first_partial_entity, _first_partial)) =
						ancestor_query
							.iter_ancestors_inclusive(action)
							.find_map(|entity| {
								path_partial_query.get(entity).ok()
							}) {
						children_query
							.iter_descendants(first_partial_entity)
							.filter_map(|entity| {
								endpoint_query.get(entity).ok()
							})
							.collect::<Vec<_>>()
					} else {
						default()
					};


				let help = EndpointHelp {
					prefix: prefix.clone(),
					ancestors,
					descendents,
				};

				commands
					.entity(route_query.requests.entity(action))
					.insert(Response::new(default(), help.render().into()));

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
	async fn help() {
		let _ = EndpointBuilder::new(|| {});
		let _ = EndpointBuilder::new(|| -> Result<(), String> { Ok(()) });

		RouterPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				EndpointBuilder::get().with_path("foobar").with_predicate(
					endpoint_help_predicate("Welcome to my CLI!"),
				)
			}))
			.oneshot_str(Request::get("/foobar").with_query_param("help", ""))
			.await
			.xpect_snapshot();
	}
}
