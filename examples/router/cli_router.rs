//! A basic example of using the beet router
use beet::prelude::*;


fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			CliPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			// The Router is a beet_flow pattern, triggering `GetOutcome`
			// and returning the Response once `Outcome` is triggered
			commands.spawn((
				CliRouter,
				// this sequence type will ensure all endpoints are checked
				// even if the previous one did not match
				InfallibleSequence,
				children![
					EndpointBuilder::get().with_handler(|| Response::ok_body(
						"hello world",
						"text/plain"
					)),
					EndpointBuilder::get().with_path("foo").with_handler(
						|| {
							Response::ok_body(
								"<div>hello foo</div>",
								// this inserts the `content-type: text/html`  header
								"text/html",
							)
						},
					),
				],
			));
		})
		.run();
}
