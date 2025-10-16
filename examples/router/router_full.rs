//! Example of using the beet router
use beet::prelude::*;

#[tokio::main]
async fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
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
				//```md
				// # Making Requests
				// We can make requests inside handlers and use the response in the html
				//```
				EndpointBuilder::get().with_path("doggo").with_handler(
					async |_: (), _: EndpointContext| {
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

						rsx!{
							<img src="{doggo}"/>
						}.xok()

					},
				),
			]));
		})
		.run();
}
