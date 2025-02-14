// use beet_flow::action_observers;
use beet_flow::prelude::*;
use bevy::prelude::*;

#[action(const_response::<T>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct ConstResponse<T: ResultPayload>(pub T);

fn const_response<T: ResultPayload>(
	req: Trigger<OnRun<T::Run>>,
	commands: Commands,
	action: Query<&ConstResponse<T>>,
) {
	let payload = action
		.get(req.action)
		.expect(&expect_action::to_have_action(&req))
		.0
		.clone();
	req.trigger_result(commands, payload);
}


fn main() {}
