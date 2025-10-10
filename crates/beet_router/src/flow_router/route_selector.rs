use std::ops::ControlFlow;

use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	actions: Query<
		'w,
		's,
		(Option<&'static PathFilter>, Option<&'static HttpMethod>),
		Or<(With<PathFilter>, With<HttpMethod>)>,
	>,
	// this will only be available until its consumed
	requests: Query<'w, 's, &'static Request>,
	agents: Query<'w, 's, &'static mut PathPartialMap>,
	parents: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
}

impl RouteQuery<'_, '_> {
	fn run_first_child(&mut self, ev: &mut On<impl ActionEvent>) -> Result {
		// insert the root path partial
		let request = self.requests.get(ev.agent())?;
		let mut path_partials = self.agents.get_mut(ev.agent())?;
		path_partials.insert_from_request(ev.action(), &request);

		let children = self.children.get(ev.action())?;
		for child in children.iter().collect::<Vec<_>>() {
			// try run and return if successful
			if self.try_run_child(ev, child)?.is_break() {
				return Ok(());
			}
		}
		Ok(())
	}

	fn try_run_child(
		&mut self,
		ev: &mut On<impl ActionEvent>,
		child: Entity,
	) -> Result<ControlFlow<()>> {
		let action = self.actions.get(child);

		// 1. Check the path filter matches
		if let Ok((Some(path_filter), _)) = action {
			let mut path_partials = self.agents.get_mut(ev.agent())?;
			let parent = self.parents.get(child)?;
			// clone the parent path partial
			let mut path_partial = path_partials
				.get(&parent.parent())
				.ok_or_else(|| bevyhow!("No path partial for parent"))?
				.clone();
			if let ControlFlow::Break(_) =
				path_partial.parse_filter(path_filter)
			{
				// path doesnt match this entity, try next child
				return ControlFlow::Continue(()).xok();
			}
			path_partials.insert(child, path_partial);
		}
		// let parent_partial =

		// we ran this child, break parent
		Ok(ControlFlow::Break(()))
	}
}


#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PreventPropagateEnd)]
pub struct RouteSelector;


fn on_start(mut ev: On<GetOutcome>, mut query: RouteQuery) -> Result {
	if let Err(_) = query.run_first_child(&mut ev) {
		// failed to run a child, just exit with Pass
		ev.trigger_next(Outcome::Pass);
	}
	Ok(())
}


fn on_next(mut ev: On<ChildEnd<Outcome>>, query: Query<&Children>) -> Result {
	let target = ev.action();
	let child = ev.child();
	// if any error, just propagate the error
	if ev.is_fail() {
		ev.propagate_child();
		return Ok(());
	}
	let children = query.get(target)?;
	let index = children
		.iter()
		.position(|x| x == child)
		.ok_or_else(|| expect_action::to_have_child(&ev, child))?;
	if index == children.len() - 1 {
		// all done, propagate the success
		ev.propagate_child();
	} else {
		// run next
		ev.trigger_next_with(children[index + 1], GetOutcome);
	}
	Ok(())
}
