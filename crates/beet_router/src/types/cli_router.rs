use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::http;
use beet_net::prelude::*;
use beet_rsx::prelude::*;


#[construct]
pub fn CliRouter() -> Result<impl Bundle> {
	Ok((Router, OnSpawn::new_async_local(handler)))
}

async fn handler(entity: AsyncEntity) -> Result {
	let req = cli_args_to_request(CliArgs::parse_env())?;
	let res = flow_route_handler(entity, req).await.into_result().await?;
	let body = res.body.into_string().await?;
	body.xprint();
	Ok(())
}


fn cli_args_to_request(args: CliArgs) -> Result<Request> {
	let path_str = args.into_path_string();
	let parts = http::Request::builder()
		.uri(path_str)
		.body(())?
		.into_parts()
		.0;
	Request::from_parts(parts, default()).xok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn into_request_simple_path() {
		cli_args_to_request(CliArgs::parse("foo bar"))
			.unwrap()
			.parts
			.uri
			.path()
			.xpect_eq("/foo/bar");
	}

	#[test]
	fn into_request_with_query() {
		let req =
			cli_args_to_request(CliArgs::parse("api users --id=123")).unwrap();
		req.parts.uri.path().xpect_eq("/api/users");
		req.parts.uri.query().xpect_some();
	}

	#[test]
	fn into_request_empty() {
		cli_args_to_request(CliArgs::parse(""))
			.unwrap()
			.parts
			.uri
			.path()
			.xpect_eq("/");
	}
}
