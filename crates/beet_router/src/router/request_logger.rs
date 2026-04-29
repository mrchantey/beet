use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[action]
#[derive(Default, Component)]
pub async fn RequestLogger(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let now = Instant::now();
	let request_parts = cx.input.0.parts().clone();
	debug!(
		"RequestLogger: Received request: \n{:#?}",
		cx.input.0.parts()
	);
	let response = cx.input.1.call(cx.input.0).await?;
	debug!(
		"RequestLogger: {} Response in {}",
		response.status(),
		time_ext::pretty_print_duration(now.elapsed())
	);
	if !response.status.is_ok() {
		// status only if ok
		debug!(
			"RequestLogger: Non-ok response:\n Request: {:#?}\n Response: {:#?}",
			request_parts,
			response.parts(),
		);
	} else {
	}

	Ok(response)
}
