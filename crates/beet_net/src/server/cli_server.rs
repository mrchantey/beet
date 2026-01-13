use crate::prelude::*;
use beet_core::prelude::*;

/// A 'server' that accepts the cli arguments and environment variables as a request,
/// exiting with the output.
#[derive(Component)]
#[component(on_add=on_add)]
pub struct CliServer;

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	world.commands().queue(move |world: &mut World| -> Result {
		world.entity_mut(entity).run_async_local(
			async move |entity| -> Result {
				let req = Request::from_cli_args(CliArgs::parse_env())?;
				let res = entity.oneshot(req).await;
				let (parts, body) = res.into_parts();
				let body = body.into_string().await?;
				let exit = match parts.status_to_exit_code() {
					Ok(()) => {
						body.xprint_display();
						AppExit::Success
					}
					Err(code) => {
						let body = if body.is_empty() {
							body
						} else {
							format!("Body:\n{}", body)
						};
						error!("Command failed\nStatus code: {code}\n{}", body);
						// TODO map http status to
						AppExit::Error(code)
					}
				};
				entity.world().write_message(exit);
				Ok(())
			},
		);
		Ok(())
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[sweet::test]
	async fn cli_server_works() {
		App::new()
			.add_plugins((MinimalPlugins, ServerPlugin))
			.spawn_then((
				CliServer,
				ExchangeSpawner::new_handler(|_, _| {
					StatusCode::IM_A_TEAPOT.into()
				}),
			))
			.run_async()
			.await
			.xpect_eq(AppExit::Error(209.try_into().unwrap()));
	}

	#[test]
	fn into_request_simple_path() {
		Request::from_cli_str("foo bar")
			.unwrap()
			.path_string()
			.xpect_eq("/foo/bar");
	}

	#[test]
	fn into_request_with_query() {
		let req = Request::from_cli_str("api users --id=123").unwrap();
		req.path_string().xpect_eq("/api/users");
		req.get_param("id").xpect_some();
	}

	#[test]
	fn into_request_empty() {
		Request::from_cli_str("")
			.unwrap()
			.path_string()
			.xpect_eq("/");
	}
}
