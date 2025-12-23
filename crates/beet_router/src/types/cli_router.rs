use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Request;
use beet_rsx::prelude::*;


#[construct]
pub fn CliRouter() -> Result<impl Bundle> {
	Ok((Router, OnSpawn::new_async_local(oneshot_cli_handler)))
}

async fn oneshot_cli_handler(entity: AsyncEntity) -> Result {
	let req = Request::from_cli_args(CliArgs::parse_env())?;
	let exit = match flow_route_handler(entity.clone(), req)
		.await
		.into_result()
		.await
	{
		Ok(res) => {
			res.body.into_string().await?.xprint_display();
			AppExit::Success
		}
		Err(err) => {
			error!("{}", err);
			AppExit::error()
		}
	};
	entity.world().write_message(exit);
	Ok(())
}


#[cfg(test)]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn into_request_simple_path() {
		Request::from_cli_args(CliArgs::parse("foo bar"))
			.unwrap()
			.parts
			.uri
			.path()
			.xpect_eq("/foo/bar");
	}

	#[test]
	fn into_request_with_query() {
		let req = Request::from_cli_args(CliArgs::parse("api users --id=123"))
			.unwrap();
		req.parts.uri.path().xpect_eq("/api/users");
		req.parts.uri.query().xpect_some();
	}

	#[test]
	fn into_request_empty() {
		Request::from_cli_args(CliArgs::parse(""))
			.unwrap()
			.parts
			.uri
			.path()
			.xpect_eq("/");
	}
}
