use beet::prelude::*;
use serde::Deserialize;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((DefaultPlugins, ServerPlugin))
		.insert_resource(ServerSettings::default()
			.with_handler(hello_server))
		.run();
}


#[derive(Deserialize)]
struct MyParams {
	name: String,
}


async fn hello_server(world: AsyncWorld, req: Request) -> Response {
	let name =
		if let Ok(params) = QueryParams::<MyParams>::from_request_ref(&req) {
			params.name.clone()
		} else {
			"User".to_string()
		};

	let count = world
		.with_resource_then::<ServerStatus, _>(|status| status.num_requests())
		.await;

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
      Uptime: {uptime:.2} seconds
      Request Count: {count}
    </pre>
  </body>
</html>
"#,
	);
	Response::ok_body(response_text, "text/html")
}
