use beet::prelude::*;
use bevy::log::LogPlugin;
use serde::Deserialize;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin,
		))
		.init_resource::<VisitCount>()
		.add_systems(Startup,|mut commands|	{
			commands.spawn(Server::default())
				.with_handler(hello_server);
		})
		.run();
}


#[derive(Deserialize)]
struct MyParams {
	name: String,
}

#[derive(Default, Resource)]
struct VisitCount(u32);

async fn hello_server(world: AsyncWorld, req: Request) -> Response {
	if req.parts.uri.path() == "/favicon.ico" {
		return Response::not_found();
	}
	let visit_count = world
		.with_resource_then::<VisitCount, _>(|mut count| {
			count.0 += 1;
			count.0
		})
		.await;


	let name =
		if let Ok(params) = QueryParams::<MyParams>::from_request_ref(&req) {
			params.name.clone()
		} else {
			"User".to_string()
		};

	let uptime = world
		.with_resource_then::<Time, _>(|time| time.elapsed_secs())
		.await;

	// the request count includes favicon get
	let response_text = format!(
		r#"
<!DOCTYPE html>
<html>
  <head>
    <style>
      body {{
     	  background-color: black;
     	  color: white;
      }}
    </style>
  </head>
  <body>
    <pre>
  Greetings {name}!
  Visit Count: {visit_count}
  Uptime: {uptime:.2} seconds
    </pre>
  </body>
</html>
"#,
	);
	Response::ok_body(response_text, "text/html")
}
