use crate::prelude::*;
use beet_core::prelude::*;

pub fn cli_server() -> impl Bundle {
	OnSpawn::new(|entity| {
		entity.run_async(async |entity| -> Result {
			let req = Request::from_cli_args(CliArgs::parse_env())?;
			let res: Response = entity.call(req).await?;
			let (parts, mut body) = res.into_parts();

			// stream body to stdout
			while let Some(chunk) = body.next().await? {
				let chunk_str = String::from_utf8_lossy(&chunk);
				cross_log_noline!("{}", chunk_str);
			}
			let exit = match parts.status_to_exit_code() {
				Ok(()) => AppExit::Success,
				Err(code) => {
					error!("Command failed\nStatus code: {code}");
					AppExit::Error(code)
				}
			};
			entity.world().write_message(exit);
			Ok(())
		});
	})
}
