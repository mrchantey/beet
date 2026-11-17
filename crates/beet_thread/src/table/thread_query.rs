use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// Read access to a [`Thread`] and its [`ThreadWindow`] from anywhere in the
/// thread's entity tree.
///
/// Conversation reads go through the in-memory window ([`Self::window`]); this
/// param only locates the thread entity and reads behavior-entity identity
/// ([`ActorRef`]), tools, and tool choice.
#[derive(SystemParam)]
pub struct ThreadQuery<'w, 's> {
	pub ancestors: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub threads:
		Query<'w, 's, (Entity, &'static Thread, &'static ThreadWindow)>,
	pub actor_refs: Query<'w, 's, &'static ActorRef>,
	pub tool_choices: Query<'w, 's, &'static ToolChoice>,
	pub tools: Query<'w, 's, (Entity, &'static ToolDefinition)>,
	pub routers: Query<'w, 's, (), With<Router>>,
	pub actors: Query<'w, 's, (), Or<(With<Actor>, With<ActorRef>)>>,
	pub path_partials: Query<'w, 's, &'static PathPartial>,
}

impl<'w, 's> ThreadQuery<'w, 's> {
	/// Walk ancestors to the [`Thread`] entity and its component.
	pub fn thread_entity(&self, entity: Entity) -> Result<(Entity, &Thread)> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| {
				self.threads
					.get(ancestor)
					.ok()
					.map(|(entity, thread, _)| (entity, thread))
			})
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity}"))
	}

	/// The [`Thread`], its entity, and its [`ThreadWindow`].
	pub fn thread_and_window(
		&self,
		entity: Entity,
	) -> Result<(Entity, &Thread, &ThreadWindow)> {
		self.ancestors
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| self.threads.get(ancestor).ok())
			.ok_or_else(|| bevyhow!("No Thread in ancestors of {entity}"))
	}

	/// The thread's window.
	pub fn window(&self, entity: Entity) -> Result<&ThreadWindow> {
		self.thread_and_window(entity).map(|(_, _, window)| window)
	}

	/// The [`ActorId`] a behavior entity acts as.
	pub fn actor_id(&self, entity: Entity) -> Result<ActorId> {
		self.actor_refs
			.get(entity)
			.map(|actor_ref| **actor_ref)
			.map_err(|_| bevyhow!("entity {entity} has no ActorRef"))
	}

	/// The [`ToolChoice`] on a behavior entity, if any.
	pub fn tool_choice(&self, entity: Entity) -> Option<&ToolChoice> {
		self.tool_choices.get(entity).ok()
	}

	/// The agent's tools in author (DFS) order, each entity with its
	/// definition: every [`ToolDefinition`] beneath `actor`, a function tool
	/// being a route of the [`Router`] beneath it, its url space.
	///
	/// ## Errors
	///
	/// Errors when a function tool sits outside that router, or the actor
	/// holds more than one, naming the fix.
	pub fn tools(
		&self,
		actor: Entity,
	) -> Result<Vec<(Entity, &ToolDefinition)>> {
		let router = self.tool_router(actor)?;
		let tools = self
			.children
			.iter_descendants_inclusive(actor)
			.filter_map(|entity| self.tools.get(entity).ok())
			.collect::<Vec<_>>();
		let stray = tools
			.iter()
			.filter(|(_, tool)| matches!(tool, ToolDefinition::Function(_)))
			.filter(|(entity, _)| {
				router.is_none_or(|router| {
					!self
						.ancestors
						.iter_ancestors_inclusive(*entity)
						.any(|ancestor| ancestor == router)
				})
			})
			.filter_map(|(_, tool)| match tool {
				ToolDefinition::Function(func) => Some(func.path()),
				ToolDefinition::Provider(_) => None,
			})
			.collect::<Vec<_>>();
		if !stray.is_empty() {
			bevybail!(
				"agent {actor} holds the tools `{}` outside a `<Router>` beneath it, \
				which is the url space its calls are answered in: mount its \
				toolsets in one, ie `<CreateActor ..><Router><StoreToolset/></Router></CreateActor>`",
				stray.join("`, `")
			);
		}
		tools.xok()
	}

	/// The url space the agent at or above `entity` is authored in, the host
	/// a tool forwards to when it acts beyond the agent's own url space, ie a
	/// perceive-act agent's `respond-multi-modal` calling the host's `drive`.
	///
	/// Named by the agent rather than by walking up until a route answers: the
	/// namespace root of the actor itself, so never the [`Router`] beneath it
	/// whose routes are its tools, however deep the tool sits in it.
	///
	/// ## Errors
	///
	/// Errors when no actor is at or above `entity`.
	pub fn agent_host(&self, entity: Entity) -> Result<Entity> {
		let actor = self
			.ancestors
			.iter_ancestors_inclusive(entity)
			.find(|ancestor| self.actors.contains(*ancestor))
			.ok_or_else(|| bevyhow!("no actor at or above {entity}"))?;
		PathPattern::namespace_root(actor, &self.ancestors, &self.path_partials)
			.xok()
	}

	/// The [`Router`] beneath `actor` answering its tool calls, if it holds
	/// one.
	///
	/// ## Errors
	///
	/// Errors when it holds more than one.
	pub fn tool_router(&self, actor: Entity) -> Result<Option<Entity>> {
		let mut routers = self
			.children
			.iter_descendants(actor)
			.filter(|entity| self.routers.contains(*entity));
		match (routers.next(), routers.next()) {
			(Some(_), Some(_)) => bevybail!(
				"agent {actor} holds more than one `<Router>`: its tools are \
				the routes of one url space, so mount every toolset in one"
			),
			(router, _) => router.xok(),
		}
	}
}
