//! One rule for what counts as a child of a behaviour, shared by every
//! control-flow node that walks children.
use crate::prelude::*;
use alloc::format;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// The children of a control-flow node, as *steps* rather than as entities.
///
/// Two questions every node used to answer for itself, answered once here.
///
/// The first is what is a child of the behaviour at all. A comment or a doctype
/// is a child of the *document*: punctuation an author wrote to annotate the
/// tree, never a step. So it is filtered before the [`BypassErrors`] policy is
/// consulted, rather than reaching around that policy — nothing is being
/// bypassed, because there was never a step there to bypass.
///
/// The second is which of those steps can serve a call, which *is* the policy's
/// question: a step with no action or with the wrong signature is skipped or
/// reported according to what the parent declared.
#[derive(SystemParam)]
pub struct BehaviourChildren<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	punctuation: Query<'w, 's, (), Punctuation>,
	metas: Query<'w, 's, &'static ActionMeta>,
	policies: Query<'w, 's, &'static BypassErrors>,
}

impl BehaviourChildren<'_, '_> {
	/// Every child of `parent` that is a candidate step.
	///
	/// A parent of nothing but punctuation has no steps, so it takes the same
	/// path a childless one does rather than reporting on zero of them.
	pub fn steps(&self, parent: Entity) -> Vec<Entity> {
		self.children
			.get(parent)
			.map(Children::iter)
			.into_iter()
			.flatten()
			.filter(|child| !self.punctuation.contains(*child))
			.collect()
	}

	/// The steps of `parent` that can serve `Action<Input, Out>`, honouring the
	/// [`BypassErrors`] it declared.
	///
	/// `node` names the caller for the error, ie `"sequence"`.
	///
	/// # Errors
	/// Errors on the first step the policy does not bypass, and on a parent
	/// that had steps but kept none of them, unless
	/// [`NONE_VALID`](ChildError::NONE_VALID) is bypassed: a parent that skipped
	/// every child would run nothing, which on a route reads as a clean exit for
	/// work that never happened.
	pub fn valid<Input, Out>(
		&self,
		node: &str,
		parent: Entity,
	) -> Result<Vec<Entity>>
	where
		Input: 'static,
		Out: 'static,
	{
		let policy = self.policies.get(parent).copied().unwrap_or_default();
		let steps = self.steps(parent);
		if steps.is_empty() {
			return Ok(Vec::new());
		}
		let mut valid = Vec::with_capacity(steps.len());
		for step in steps.iter().copied() {
			if policy.serves::<Input, Out>(
				node,
				step,
				self.metas.get(step).ok(),
			)? {
				valid.push(step);
			}
		}
		if valid.is_empty() && !policy.contains(ChildError::NONE_VALID) {
			bevybail!(
				"{node} {parent} skipped all {} of its children, none serve Action<{}, {}>:{}",
				steps.len(),
				core::any::type_name::<Input>(),
				core::any::type_name::<Out>(),
				self.describe(steps.iter().copied())
			);
		}
		Ok(valid)
	}

	/// The first step of `parent` that can serve `Action<Input, Out>`, the
	/// downward dispatch hop: a parent hands its call to the first child that
	/// can take it, ignoring config-only and differently-shaped children.
	///
	/// # Errors
	/// Errors when no step matches the signature, listing each one's signatures.
	pub fn first_matching<Input, Out>(&self, parent: Entity) -> Result<Entity>
	where
		Input: 'static,
		Out: 'static,
	{
		let steps = self.steps(parent);
		steps
			.iter()
			.copied()
			.find(|step| {
				self.metas
					.get(*step)
					.is_ok_and(|meta| meta.matches::<Input, Out>())
			})
			.ok_or_else(|| {
				bevyhow!(
					"no child of {parent} matches Action<{}, {}>:{}",
					core::any::type_name::<Input>(),
					core::any::type_name::<Out>(),
					self.describe(steps.iter().copied())
				)
			})
	}

	/// The single step of `parent`, for a node that drives one child.
	///
	/// [`None`] when it has none, which those nodes treat as nothing to do
	/// rather than as a failure.
	pub fn only(&self, parent: Entity) -> Option<Entity> {
		self.steps(parent).first().copied()
	}

	/// The steps of `parent` that can serve `Action<Input, Out>`, as an action
	/// reaches them.
	///
	/// A `SystemParam` cannot be held across an await, so every async caller
	/// goes through the cached system rather than building its own query.
	///
	/// # Errors
	/// Propagates [`valid`](Self::valid)'s.
	pub async fn valid_for<Input, Out>(
		world: &AsyncWorld,
		parent: Entity,
		node: &'static str,
	) -> Result<Vec<Entity>>
	where
		Input: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	{
		world
			.run_system_cached_with(
				valid_children_system::<Input, Out>,
				(parent, node),
			)
			.await?
	}

	/// The steps of `parent`, as an action reaches them.
	pub async fn steps_for(world: &AsyncWorld, parent: Entity) -> Vec<Entity> {
		world
			.run_system_cached_with(behaviour_children, parent)
			.await
			.unwrap_or_default()
	}

	/// The single step of `parent`, as an action reaches them.
	pub async fn only_for(
		world: &AsyncWorld,
		parent: Entity,
	) -> Option<Entity> {
		world
			.run_system_cached_with(only_behaviour_child, parent)
			.await
			.ok()
			.flatten()
	}

	/// Why each of `steps` could not serve a call, one indented line each: its
	/// action's signatures, else that it carries no action at all.
	fn describe(&self, steps: impl Iterator<Item = Entity>) -> String {
		steps
			.map(|step| match self.metas.get(step) {
				Ok(meta) => format!("\n  {step}: {}", meta.signatures()),
				Err(_) => format!("\n  {step}: no action"),
			})
			.collect::<String>()
	}
}

/// The children of `parent` that can serve `Action<Input, Out>`, as a cached
/// system.
fn valid_children_system<Input, Out>(
	In((parent, node)): In<(Entity, &'static str)>,
	children: BehaviourChildren,
) -> Result<Vec<Entity>>
where
	Input: 'static,
	Out: 'static,
{
	children.valid::<Input, Out>(node, parent)
}

/// The steps of `parent`, as a cached system.
fn behaviour_children(
	In(parent): In<Entity>,
	children: BehaviourChildren,
) -> Vec<Entity> {
	children.steps(parent)
}

/// The single step of `parent`, as a cached system.
fn only_behaviour_child(
	In(parent): In<Entity>,
	children: BehaviourChildren,
) -> Option<Entity> {
	children.only(parent)
}

/// Selects the first child whose [`ActionMeta`] [`matches`](ActionMeta::matches)
/// `(Input, Out)`, as a cached system.
///
/// # Errors
/// Errors when no child matches the signature, listing each child's signatures.
pub fn first_matching_child<Input, Out>(
	In(parent): In<Entity>,
	children: BehaviourChildren,
) -> Result<Entity>
where
	Input: 'static,
	Out: 'static,
{
	children.first_matching::<Input, Out>(parent)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A comment is a child of the document, not of the behaviour, so it is
	/// never a step and no node has to be told to skip it.
	#[beet_core::test]
	fn punctuation_is_not_a_step() {
		let mut world = World::new();
		let parent = world
			.spawn(children![
				Comment::new("a note"),
				Doctype::new("html"),
				Name::new("real"),
			])
			.id();
		world
			.run_system_once_with(super::behaviour_children, parent)
			.unwrap()
			.len()
			.xpect_eq(1);
	}

	/// A parent of nothing but punctuation reads as childless rather than as a
	/// parent that skipped everything.
	#[beet_core::test]
	fn punctuation_only_reads_as_childless() {
		let mut world = World::new();
		let parent = world.spawn(children![Comment::new("a note")]).id();
		let steps: Result<Vec<Entity>> = world
			.run_system_once_with(
				super::valid_children_system::<(), Outcome>,
				(parent, "sequence"),
			)
			.unwrap();
		steps.unwrap().xpect_empty();
	}
}
