// use beet_flow::action_observers;
use crate::prelude::*;
use bevy::prelude::*;



pub trait ConstResponseForRequest: Request {
	fn fixed_response(response: Self::Res) -> ConstResponse<Self> {
		ConstResponse(response)
	}
}
impl<T> ConstResponseForRequest for T where T: Request {}

#[action(const_response::<R>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct ConstResponse<R: Request>(pub R::Res);

fn const_response<R: Request>(
	req: Trigger<OnRequest<R>>,
	mut commands: Commands,
	action: Query<&ConstResponse<R>>,
) {
	let payload = action
		.get(req.action)
		.expect(&expect_action::to_have_action(req.action))
		.0
		.clone();
	commands
		.entity(req.action)
		.trigger(req.into_response(payload));
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());

		let observed = observe_triggers::<OnResponse<Run>>(app.world_mut());
		let entity = app
			.world_mut()
			.spawn(Run::fixed_response(Err(())))
			.flush_trigger(OnRequest::new(Run))
			.id();

		expect(&observed).to_have_been_called_times(1);
		expect(&observed).to_have_returned_nth_with(
			0,
			&OnResponse::new_with_action(entity, Err(())),
		);
	}
}
