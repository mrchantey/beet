//! Running a [`Group`]: the same ordered members, forward or backwards.
use crate::prelude::*;
use alloc::format;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Runs a [`Group`]'s members in order, threading `Input` through each exactly
/// as [`Sequence`] threads it through children.
///
/// Where a sequence's steps are its children, a group's are its [`Members`], so
/// a step may be authored anywhere and still run here. Nesting flattens: a
/// member that is itself a group contributes its own members in place, under its
/// own [`BypassErrors`] policy.
///
/// Direction belongs to the RUN, never to the group. `reverse` runs the
/// flattened list backwards, which is the one place in the system where "a
/// teardown is a convergence read the other way" is written down; a group is
/// therefore always authored in convergence order:
///
/// ```bsx
/// <Group bx:ref="down">
///     <RemoveTheStore/>   // converges first, so it tears down last
///     <RemoveTheThing/>
/// </Group>
/// <Route path="destroy" {RunGroup{group: $down, reverse: true}}/>
/// ```
///
/// Returns the first [`Outcome::Fail`] immediately, or [`Outcome::Pass`] with
/// the final input if every member passed. Under `continue_on_failure` it runs
/// every member regardless and reports the collected failures at the end, which
/// is what a `--force` teardown wants: as much removed as possible, and an
/// honest account of what was not.
///
/// A failing member takes the threaded input with it (a failure answers with
/// `Output`, not with the input it was handed), so a continuing run resumes from
/// a fresh [`Default`] one. That is why it is a teardown mode: every step of a
/// teardown resolves what it needs from the world, so a request that has lost
/// its params still tears the stack down. A run that threads meaning through its
/// input should fail fast instead.
///
/// ## Errors
///
/// Errors depending on [`ChildError`] flags when a member has:
/// - no [`ActionMeta`]
/// - incompatible [`ActionMeta`] signature
///
/// ..and always when `group` names an entity that is not a [`Group`], or when a
/// group contains itself.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn RunGroup<Input = (), Output = ()>(
	/// The group to run, this entity when absent.
	#[field]
	group: Option<Entity>,
	/// Run the flattened member list backwards.
	#[field]
	reverse: bool,
	/// Run every member even after one fails, reporting the failures at the end.
	#[field]
	continue_on_failure: bool,
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync + Default,
	Output: 'static + Send + Sync,
{
	let group = group.unwrap_or(cx.id());
	let world = cx.world();
	let mut members =
		GroupMembers::flatten_for::<Input, Outcome<Input, Output>>(
			&world,
			group,
			cx.id(),
		)
		.await?;
	if reverse {
		members.reverse();
	}

	let mut input = cx.input;
	let mut failures = Vec::new();
	for member in members {
		let failure = match world
			.entity(member)
			.call::<Input, Outcome<Input, Output>>(input)
			.await
		{
			Ok(Outcome::Pass(next)) => {
				input = next;
				continue;
			}
			Ok(Outcome::Fail(output)) if !continue_on_failure => {
				return Ok(Outcome::Fail(output));
			}
			Err(err) if !continue_on_failure => return Err(err),
			Ok(Outcome::Fail(_)) => {
				input = Input::default();
				format!("{member} failed")
			}
			Err(err) => {
				input = Input::default();
				format!("{member}: {err}")
			}
		};
		error!("group {group}: continuing past a failed member. {failure}");
		failures.push(failure);
	}
	if !failures.is_empty() {
		bevybail!(
			"group {group} ran every member; {} failed:\n  {}",
			failures.len(),
			failures.join("\n  ")
		);
	}
	Ok(Outcome::Pass(input))
}

impl<Input, Output> RunGroup<Input, Output>
where
	Input: 'static + Send + Sync + Default,
	Output: 'static + Send + Sync,
{
	/// Run `group` forward, ie in convergence order.
	pub fn new(group: Entity) -> Self {
		Self {
			group: Some(group),
			..default()
		}
	}

	/// Run `group` backwards, which is what a teardown is.
	pub fn reversed(group: Entity) -> Self {
		Self {
			reverse: true,
			..Self::new(group)
		}
	}

	/// Run every member even after one fails, reporting the failures at the end.
	pub fn with_continue_on_failure(mut self, value: bool) -> Self {
		self.continue_on_failure = value;
		self
	}
}

/// The run order of a [`Group`], flattened.
///
/// A `SystemParam` cannot be held across an await, so the action reaches it
/// through [`flatten_for`](Self::flatten_for)'s cached system.
#[derive(SystemParam)]
pub struct GroupMembers<'w, 's> {
	members: Query<'w, 's, &'static Members>,
	groups: Query<'w, 's, (), With<Group>>,
	/// The step rules every control-flow node shares: what counts as a step at
	/// all, and which of them the [`BypassErrors`] policy keeps.
	steps: BehaviourChildren<'w, 's>,
}

impl GroupMembers<'_, '_> {
	/// The depth-first run order of `group`: every member in enrollment order,
	/// with a member that is itself a group replaced in place by its own.
	///
	/// Flattening rather than recursing at call time is what makes `reverse` one
	/// operation on one list, and it is the same order either way
	/// (`rev(A ++ B) = rev(B) ++ rev(A)`).
	///
	/// A group declares its own [`BypassErrors`]; `default_policy` covers one
	/// that did not, so a route can say once that config-only members are to be
	/// skipped without every group in the document repeating it.
	///
	/// # Errors
	/// Errors when `group` is not a [`Group`], when a group contains itself, and
	/// on any member the governing [`BypassErrors`] does not excuse.
	pub fn flatten<Input, Out>(
		&self,
		group: Entity,
		default_policy: BypassErrors,
	) -> Result<Vec<Entity>>
	where
		Input: 'static,
		Out: 'static,
	{
		if !self.groups.contains(group) {
			bevybail!("{group} is not a `Group`, so it has no members to run");
		}
		let mut order = Vec::new();
		self.flatten_into::<Input, Out>(
			group,
			default_policy,
			&mut order,
			&mut Vec::new(),
		)?;
		Ok(order)
	}

	/// Append `group`'s run order to `order`, with `open` the groups already
	/// being flattened, so a cycle reports rather than overflowing the stack.
	fn flatten_into<Input, Out>(
		&self,
		group: Entity,
		default_policy: BypassErrors,
		order: &mut Vec<Entity>,
		open: &mut Vec<Entity>,
	) -> Result
	where
		Input: 'static,
		Out: 'static,
	{
		if open.contains(&group) {
			bevybail!("group {group} is a member of itself");
		}
		open.push(group);
		let steps = self.steps.steps_of(
			self.members
				.get(group)
				.map(Members::iter)
				.into_iter()
				.flatten(),
		);
		// a nested group is a step whatever action it carries: it contributes its
		// members rather than being called, so the policy never sees it.
		let leaves = steps
			.iter()
			.copied()
			.filter(|step| !self.groups.contains(*step))
			.collect::<Vec<_>>();
		let valid = self.steps.valid_with::<Input, Out>(
			"group",
			group,
			leaves,
			self.steps.policy(group).unwrap_or(default_policy),
		)?;
		for step in steps {
			match self.groups.contains(step) {
				true => self.flatten_into::<Input, Out>(
					step,
					default_policy,
					order,
					open,
				)?,
				false if valid.contains(&step) => order.push(step),
				false => {}
			}
		}
		open.pop();
		Ok(())
	}

	/// The run order of `group`, as an action reaches it.
	///
	/// # Errors
	/// Propagates [`flatten`](Self::flatten)'s.
	pub async fn flatten_for<Input, Out>(
		world: &AsyncWorld,
		group: Entity,
		runner: Entity,
	) -> Result<Vec<Entity>>
	where
		Input: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	{
		world
			.run_system_cached_with(
				flatten_group_system::<Input, Out>,
				(group, runner),
			)
			.await?
	}
}

/// The run order of `group`, as a cached system, with `runner`'s
/// [`BypassErrors`] the default for any group that declared none of its own.
fn flatten_group_system<Input, Out>(
	In((group, runner)): In<(Entity, Entity)>,
	members: GroupMembers,
) -> Result<Vec<Entity>>
where
	Input: 'static,
	Out: 'static,
{
	let default_policy = members.steps.policy(runner).unwrap_or_default();
	members.flatten::<Input, Out>(group, default_policy)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An action appending `name` to `log`, the step every ordering test is made
	/// of.
	fn say(log: Store<Vec<&'static str>>, name: &'static str) -> impl Bundle {
		Action::<(), Outcome>::new_pure(move |_: ActionContext| {
			log.push(name);
			Outcome::PASS.xok()
		})
	}

	fn fails(log: Store<Vec<&'static str>>, name: &'static str) -> impl Bundle {
		Action::<(), Outcome>::new_pure(move |_: ActionContext| {
			log.push(name);
			Outcome::FAIL.xok()
		})
	}

	/// Run `action` from an entity of its own, as a route endpoint does.
	async fn run(world: &mut World, action: RunGroup) -> Result<Outcome> {
		world.spawn(action).call::<(), Outcome>(()).await
	}

	#[beet_core::test]
	async fn runs_members_forward_and_reversed() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let group = world
			.spawn((Group, children![
				say(log.clone(), "one"),
				say(log.clone(), "two"),
			]))
			.flush();
		run(&mut world, RunGroup::new(group))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["one", "two"]);

		log.set(Vec::new());
		run(&mut world, RunGroup::reversed(group))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["two", "one"]);
	}

	/// The host entity is the default group, so a `RunGroup` sitting on one runs
	/// its own members.
	#[beet_core::test]
	async fn a_group_runs_itself() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		world
			.spawn((Group, RunGroup::<(), ()>::default(), children![say(
				log.clone(),
				"one"
			),]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["one"]);
	}

	/// A nested group contributes its members in place, so the flattened order
	/// reversed is the same list read backwards, not the groups swapped.
	#[beet_core::test]
	async fn nested_groups_flatten() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let outer = world
			.spawn((Group, children![
				say(log.clone(), "one"),
				(Group, children![
					say(log.clone(), "two"),
					say(log.clone(), "three"),
				]),
				say(log.clone(), "four"),
			]))
			.flush();
		run(&mut world, RunGroup::new(outer))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["one", "two", "three", "four"]);

		log.set(Vec::new());
		run(&mut world, RunGroup::reversed(outer))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["four", "three", "two", "one"]);
	}

	/// Enrollment is by reference, so a member declared outside the group still
	/// runs, in the order it enrolled.
	#[beet_core::test]
	async fn remote_members_run_in_enrollment_order() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let group = world.spawn(Group).flush();
		world.spawn((say(log.clone(), "one"), MemberOf(group)));
		world.spawn((say(log.clone(), "two"), MemberOf(group)));
		world.flush();
		run(&mut world, RunGroup::new(group))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["one", "two"]);
	}

	#[beet_core::test]
	async fn an_empty_group_is_a_clean_no_op() {
		let mut world = AsyncPlugin::world();
		let group = world.spawn(Group).flush();
		run(&mut world, RunGroup::new(group))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn running_a_non_group_errors() {
		let mut world = AsyncPlugin::world();
		let plain = world.spawn_empty().flush();
		run(&mut world, RunGroup::new(plain))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("is not a `Group`");
	}

	#[beet_core::test]
	async fn a_failing_member_stops_the_run() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let group = world
			.spawn((Group, children![
				say(log.clone(), "one"),
				fails(log.clone(), "two"),
				say(log.clone(), "three"),
			]))
			.flush();
		run(&mut world, RunGroup::new(group))
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
		log.get().xpect_eq(vec!["one", "two"]);
	}

	/// The `--force` teardown mode: every member runs, and the run still reports
	/// what failed rather than exiting clean.
	#[beet_core::test]
	async fn continue_on_failure_runs_everything_and_reports() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let group = world
			.spawn((Group, children![
				fails(log.clone(), "one"),
				say(log.clone(), "two"),
				fails(log.clone(), "three"),
			]))
			.flush();
		run(
			&mut world,
			RunGroup::new(group).with_continue_on_failure(true),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("2 failed");
		log.get().xpect_eq(vec!["one", "two", "three"]);
	}

	/// A member with no action at all (a config block) is skipped under the
	/// policy, exactly as a sequence's child is.
	#[beet_core::test]
	async fn a_member_with_no_action_is_bypassable() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let group = world
			.spawn((Group, BypassErrors(ChildError::NO_ACTION), children![
				Name::new("config-only"),
				say(log.clone(), "one")
			]))
			.flush();
		run(&mut world, RunGroup::new(group))
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["one"]);
	}

	/// A declaration that owns both halves of a thing's lifecycle spawns them
	/// from one place, so the two groups' orders are projections of one insert
	/// order and cannot drift: the teardown IS the convergence read backwards.
	#[beet_core::test]
	async fn a_paired_declaration_cannot_drift() {
		fn pair(
			log: Store<Vec<&'static str>>,
			name: &'static str,
			up: Entity,
			down: Entity,
		) -> impl Bundle {
			children![
				(say(log.clone(), name), MemberOf(up)),
				(say(log, name), MemberOf(down)),
			]
		}
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let up = world.spawn(Group).flush();
		let down = world.spawn(Group).flush();
		world.spawn(pair(log.clone(), "first", up, down));
		world.spawn(pair(log.clone(), "second", up, down));
		world.flush();

		run(&mut world, RunGroup::new(up)).await.unwrap();
		log.get().xpect_eq(vec!["first", "second"]);
		log.set(Vec::new());
		run(&mut world, RunGroup::reversed(down)).await.unwrap();
		log.get().xpect_eq(vec!["second", "first"]);
	}

	/// A group with no policy of its own inherits the run's, which is how a
	/// route says once that config-only members are to be skipped.
	#[beet_core::test]
	async fn the_run_supplies_the_default_policy() {
		let log = Store::default();
		let mut world = AsyncPlugin::world();
		let group = world
			.spawn((Group, children![
				Name::new("config-only"),
				say(log.clone(), "one"),
			]))
			.flush();
		world
			.spawn((
				RunGroup::<(), ()>::new(group),
				BypassErrors(ChildError::NO_ACTION),
			))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		log.get().xpect_eq(vec!["one"]);
	}

	/// Input threads member to member, in whichever direction the run goes.
	#[beet_core::test]
	async fn input_threads_in_both_directions() {
		fn append(suffix: &'static str) -> impl Bundle {
			Action::<String, Outcome<String, ()>>::new_pure(
				move |cx: ActionContext<String>| {
					Outcome::Pass(format!("{}{suffix}", cx.input)).xok()
				},
			)
		}
		async fn thread(reverse: bool) -> String {
			let mut world = AsyncPlugin::world();
			let group = world
				.spawn((Group, children![append("a"), append("b")]))
				.flush();
			let action = RunGroup::<String, ()> {
				reverse,
				..RunGroup::new(group)
			};
			match world
				.spawn(action)
				.call::<String, Outcome<String, ()>>("_".into())
				.await
				.unwrap()
			{
				Outcome::Pass(value) => value,
				Outcome::Fail(_) => unreachable!(),
			}
		}
		thread(false).await.xpect_eq("_ab");
		thread(true).await.xpect_eq("_ba");
	}
}
