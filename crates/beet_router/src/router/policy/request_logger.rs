use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Middleware dumping both sides of an exchange whose response is not ok, at
/// `debug`. Every exchange already logs one `info` line, its method, path,
/// status and duration (`ServerPlugin`'s exchange stats), so this adds only
/// what that line cannot carry.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = MiddlewareList::<Request, Response>::on_add::<Self>)]
pub async fn RequestLogger(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let request_parts = cx.input.0.parts().clone();
	let response = cx.input.1.call(cx.input.0).await?;
	if !response.status.is_ok() {
		debug!(
			"RequestLogger: Non-ok response:\n Request: {:#?}\n Response: {:#?}",
			request_parts,
			response.parts(),
		);
	}
	Ok(response)
}
