//! Example of using the beet router
use beet::prelude::*;

fn main() { async_ext::block_on(main_async()); }
async fn main_async() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.insert_resource(pkg_config!())
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn(default_router(
				EndWith(Outcome::Pass),
				(InfallibleSequence, children![
					EndpointBuilder::get().with_handler(|| Response::ok_body(
						"hello world",
						"text/plain"
					)),
					EndpointBuilder::get().with_path("foo").with_handler(
						|| { Response::ok_body("hello foo", "text/plain") },
					),
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
							let doggo_url = res["message"].as_str().unwrap();

							rsx! {
								<img src=doggo_url/>
							}
							.xok()
						},
					),
				]),
				EndWith(Outcome::Pass),
			));
		})
		.run();
}
