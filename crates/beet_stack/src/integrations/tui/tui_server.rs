use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Default, Component)]
pub struct TuiServer;


pub fn tui_server() -> impl Bundle {
	(
		TuiServer,
		tui_render_tool(),
		OnSpawn::new_async(async |entity| -> Result {
			// Dispatch CLI args as the initial request, rendering the
			// root content when no args are provided.
			let initial_req = Request::from_cli_args(CliArgs::parse_env())?;
			entity
				.call::<_, Response>(initial_req)
				.await?
				.into_result()
				.await?;


			Ok(())
		}),
	)
}
