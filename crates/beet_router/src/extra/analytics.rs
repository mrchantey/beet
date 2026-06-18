use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::In;

/// A `POST /analytics` route that parses an [`AnalyticsEvent`] from the JSON
/// body and triggers it as a bevy event for any registered observer to record.
pub fn analytics_handler() -> impl Bundle {
	(
		exchange_route("analytics", AnalyticsHandler),
		HttpMethod::Post,
	)
}

#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
fn AnalyticsHandler(
	cx: In<ActionContext<serde_json::Value>>,
	mut commands: Commands,
) -> Result {
	let event = AnalyticsEvent::parse(cx.input.clone())?;
	commands.trigger(event);
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[derive(Resource, Default)]
	struct AnalyticsHits(u32);

	#[beet_core::test]
	async fn accepts_post_and_triggers_event() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.init_resource::<AnalyticsHits>();
		world.add_observer(
			|_ev: On<AnalyticsEvent>, mut hits: ResMut<AnalyticsHits>| {
				hits.0 += 1;
			},
		);
		// `default_router` already wires `analytics_handler()` under json + std.
		let root = world.spawn(default_router()).flush();
		let payload = r#"{"event_type":"click","client_timestamp":1,"event_data":{},"session_data":{}}"#;

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::with_json_str(
				"analytics",
				payload,
			))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);

		world.resource::<AnalyticsHits>().0.xpect_eq(1);
	}
}
