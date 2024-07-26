use bevy::prelude::*;

#[derive(Event)]
struct OnRun;
#[derive(Event)]
struct OnChildResult {
	pub result: OnRunResult,
	pub child: Entity,
}
#[derive(Event, PartialEq)]
enum OnRunResult {
	Success,
	Failure,
}

fn sequence_start(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<&Children>,
) {
	if let Ok(children) = query.get(trigger.entity()) {
		if let Some(first_child) = children.iter().next() {
			commands.trigger_targets(OnRun, *first_child);
		}
	}
}
fn sequence_next(
	trigger: Trigger<OnChildResult>,
	mut commands: Commands,
	query: Query<&Children>,
) {
	if trigger.event().result == OnRunResult::Failure {
		commands.trigger_targets(OnRunResult::Failure, trigger.entity());
		return;
	}
	if let Ok(children) = query.get(trigger.entity()) {
		let index = children
			.iter()
			.position(|&x| x == trigger.event().child)
			.expect("Only children may trigger OnChildResult");
		if index == children.len() - 1 {
			println!("Sequence complete!");
			commands.trigger_targets(OnRunResult::Success, trigger.entity());
		} else {
			commands.trigger_targets(OnRun, children[index + 1]);
		}
	}
}

fn log_on_run(trigger: Trigger<OnRun>, names: Query<&Name>) {
	let name = names
		.get(trigger.entity())
		.map(|n| n.as_str())
		.unwrap_or("");
	println!("Running: {name}");
}

fn succeed_on_run(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	parents: Query<&Parent>,
) {
	commands.trigger_targets(OnRunResult::Success, trigger.entity());
	if let Some(parent) = parents.get(trigger.entity()).ok() {
		commands.trigger_targets(
			OnChildResult {
				result: OnRunResult::Success,
				child: trigger.entity(),
			},
			parent.get(),
		);
	}
}

fn main() {
	let mut world = World::new();

	world.observe(log_on_run);

	let entity = world
		.spawn(Name::new("Root"))
		.with_children(|parent| {
			parent.spawn(Name::new("Hello")).observe(succeed_on_run);
			parent.spawn(Name::new("World")).observe(succeed_on_run);
		})
		.observe(sequence_start)
		.observe(sequence_next)
		.id();

	init_observers(&mut world);

	world.trigger_targets(OnRun, entity);
	world.flush();
}


/// its a bug, without this other observers dont run?
fn init_observers(world: &mut World) {
	world.observe(|_: Trigger<OnAdd>| {
		// println!("here");
	});
}
