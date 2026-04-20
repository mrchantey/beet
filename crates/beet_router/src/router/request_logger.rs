use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[action]
#[derive(Default, Component)]
pub async fn RequestLogger(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let now = Instant::now();
	debug!("RequestLogger: Received request: \n{:?}", cx.input.0);
	let response = cx.input.1.call(cx.input.0).await?;
	if response.status.is_ok() {
		// status only if ok
		debug!(
			"RequestLogger: Response in (took {:?}):\n {:?} ",
			response.status(),
			now.elapsed()
		);
	} else {
		debug!(
			"RequestLogger: Response in (took {:?}):\n {:?} ",
			response,
			now.elapsed()
		);
	}

	Ok(response)
}
