use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Debug, Default)]
pub struct ActionChanges {
	pub created: Vec<ActionId>,
	pub modified: Vec<ActionId>,
}



#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship(relationship_target = Actions)]
pub struct ActionOf(Entity);

impl ActionOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship_target(relationship = ActionOf,linked_spawn)]
pub struct Actions(Vec<Entity>);


#[derive(Debug)]
pub struct ActionJoin {
	pub action: Action,
	pub actor: Actor,
	pub thread: Thread,
}


impl ActionChanges {
	pub fn is_empty(&self) -> bool {
		self.created.is_empty() && self.modified.is_empty()
	}

	/// All action ids that were either created or modified
	pub fn all_actions(&self) -> Vec<ActionId> {
		self.created
			.iter()
			.chain(self.modified.iter())
			.copied()
			.collect()
	}
}

/// Action created event, runs before [`EntityActionCreated`] and [`ActionUpdated`]
#[derive(Event)]
pub struct ActionCreated {
	pub action: ActionId,
	pub thread: ThreadId,
	pub actor: ActorId,
}

#[derive(Event)]
pub struct ActionUpdated {
	pub action: ActionId,
	pub thread: ThreadId,
	pub actor: ActorId,
}

#[derive(Event)]
pub struct ResponseComplete {
	/// The openresponses id for this response
	pub id: String,
	pub interrupted: bool,
}


#[derive(SystemParam)]
pub struct ActionQuery<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub actor_query: Query<'w, 's, (Entity, &'static ActorId)>,
	pub thread_query: Query<'w, 's, (Entity, &'static ThreadId)>,
}
