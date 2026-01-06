//! Example of a basic server with hand-rolled routing and templating
use beet::prelude::*;
use serde::Deserialize;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
		))
		.init_resource::<VisitCounter>()
		.add_observer(handler)
		.run();
}

#[derive(Deserialize)]
struct MyParams {
	name: String,
}

#[derive(Default, Resource)]
struct VisitCounter(u32);

fn handler(
	ev: On<Insert, Request>,
	mut commands: Commands,
	requests: Query<&RequestMeta>,
	time: Res<Time>,
	mut visit_counter: ResMut<VisitCounter>,
) -> Result {
	let request = requests.get(ev.event_target())?;
	let path = request.path_string();
	// our diy router, only match root path
	if path != "/" {
		commands
			.entity(ev.event_target())
			.insert(Response::from_status_body(
				StatusCode::NOT_FOUND,
				format!("Path not found: {}", path),
				"text/plain",
			));
		return Ok(());
	}
	visit_counter.0 += 1;
	let num_visits = visit_counter.0;

	let name = if let Ok(params) =
		QueryParams::<MyParams>::from_request_meta(&request)
	{
		params.name.clone()
	} else {
		"User".to_string()
	};

	let uptime = time.elapsed_secs();

	let special_message = if num_visits % 7 == 0 {
		format!("<p>Congratulations you are visitor number {num_visits}!</p>")
	} else {
		default()
	};
	// simple templating with format!
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
  Visit Count: {num_visits}
  Uptime: {uptime:.2} seconds
    </pre>
  {special_message}
  </body>
</html>
"#,
	);
	commands
		.entity(ev.event_target())
		.insert(Response::ok_body(response_text, "text/html"));
	Ok(())
}
