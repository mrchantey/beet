use beet::exports::axum::Json;
use beet::prelude::*;



/// rejects any negative number
pub async fn post(Json(a): Json<i32>) -> ActionResult<Json<u32>> {
	if a >= 0 {
		Ok(Json(a as u32))
	} else {
		Err(anyhow::anyhow!("expected positive number, received {a}")
			.into_action_error())
	}
}
