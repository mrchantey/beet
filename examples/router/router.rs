//! Example of using the beet router
use beet::prelude::*;

// boo tokio todo replace reqwest
#[tokio::main]
async fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			FlowRouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((RouteServer, InfallibleSequence, children![
				EndpointBuilder::get().with_handler(|| Response::ok_body(
					"hello world",
					"text/plain"
				)),
				EndpointBuilder::get().with_path("foo").with_handler(|| {
					Response::ok_body("hello foo", "text/plain")
				},),
				EndpointBuilder::get().with_path("doggo").with_handler(
					async |_: (), _: EndpointContext| -> Result<Response> {
						let res = Request::get(
							"https://dog.ceo/api/breeds/image/random",
						)
						.send()
						.await?
						.into_result()
						.await?
						.json::<serde_json::Value>()
						.await
						.unwrap();
						let doggo = res["message"].as_str().unwrap();

						Response::ok_body(
							format!(r#"<img src="{doggo}"/>"#),
							"text/html",
						)
						.xok()
					},
				),
			]));
		})
		.run();
}
