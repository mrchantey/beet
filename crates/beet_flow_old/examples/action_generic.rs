// use beet_flow::action_observers;
use beet_flow::prelude::*;
use beet_core::prelude::*;

#[action(const_response::<T>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct ConstResponse<T: ResultPayload>(pub T);

fn const_response<T: ResultPayload>(
	req: On<OnRun<T::Run>>,
	mut commands: Commands,
	action: Query<&ConstResponse<T>>,
) {
	let payload = action
		.get(req.action)
		.expect(&expect_action::to_have_action(&req))
		.0
		.clone();
	req.trigger_result(&mut commands, payload);
}


fn main() {}
