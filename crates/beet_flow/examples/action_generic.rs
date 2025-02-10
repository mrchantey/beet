// use beet_flow::action_observers;
use beet_flow::prelude::*;
use bevy::prelude::*;


#[action(const_response::<R>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct ConstResponse<R: Request>(pub R::Res);

fn const_response<R: Request>(
	req: Trigger<On<R>>,
	commands: Commands,
	action: Query<&ConstResponse<R>>,
) {
	let payload = action
		.get(req.action)
		.expect(&expect_action::to_have_action(&req))
		.0
		.clone();
	req.trigger_response(commands, payload);
}


fn main() {}
