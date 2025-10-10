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
		.add_systems(Startup, setup)
		.run();
}
fn setup(mut commands: Commands) {
	commands.spawn((Server::default().with_handler(handler), VisitCount(0)));
}

#[derive(Deserialize)]
struct MyParams {
	name: String,
}

#[derive(Default, Component)]
struct VisitCount(u32);

async fn handler(entity: AsyncEntity, req: Request) -> Response {
	let path = req.parts.uri.path();
	// our diy router :)
	if path != "/" {
		return Response::from_status_body(
			StatusCode::NOT_FOUND,
			format!("Path not found: {}", path),
			"text/plain",
		);
	}
	let visit_count = entity
		.get_mut::<VisitCount, _>(|mut count| {
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

	let uptime = entity
		.world()
		.with_resource_then::<Time, _>(|time| time.elapsed_secs())
		.await;

	let special_message = if visit_count % 7 == 0 {
		format!("<p>Congratulations you are visitor number {visit_count}!</p>")
	} else {
		default()
	};

	// the request count includes favicon get
	let response_text = format!(
		r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Beet Server</title>
    <style>
      body {{
      font-family: system-ui, sans-serif;
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
  {special_message}
  </body>
</html>
"#,
	);
	Response::ok_body(response_text, "text/html")
}
