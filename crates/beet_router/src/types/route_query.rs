use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::TemplateOf;
use beet_flow::prelude::*;
use bevy::reflect::Typed;

#[derive(SystemParam)]
pub struct RouteQuery<'w, 's> {
	pub agents: AgentQuery<'w, 's, &'static RequestMeta>,
	pub parents: Query<'w, 's, &'static ChildOf>,
	pub children: Query<'w, 's, &'static Children>,
	pub path_partials: Query<'w, 's, &'static PathPartial>,
	pub params_partials: Query<'w, 's, &'static ParamsPartial>,
	endpoint_trees: Query<'w, 's, &'static EndpointTree>,
	templates: Query<'w, 's, &'static TemplateOf>,
	action_ofs: Query<'w, 's, &'static ActionOf>,
}

impl RouteQuery<'_, '_> {
	pub fn request_meta(&self, action: Entity) -> Result<&RequestMeta> {
		self.agents.get(action)?.xok()
	}

	pub fn path(&self, action: Entity) -> Result<&Vec<String>> {
		self.agents.get(action)?.path().xok()
	}
	pub fn method(&self, action: Entity) -> Result<HttpMethod> {
		self.agents.get(action)?.method().xok()
	}

	pub fn path_match(&self, action: Entity) -> Result<PathMatch> {
		let path = self.path(action)?;
		let pattern = PathPattern::collect(action, &self)?;
		pattern.parse_path(path)?.xok()
	}

	pub fn dyn_segment(&mut self, action: Entity, key: &str) -> Result<String> {
		self.path_match(action)?
			.dyn_map
			.get(key)
			.map(|key| key.clone())
			.ok_or_else(|| bevyhow!("key not found: {}", key))
	}

	pub async fn dyn_segment_async(
		action: AsyncEntity,
		key: &str,
	) -> Result<String> {
		let key = key.to_string();
		Self::with_async(action, move |query, entity| {
			query.dyn_segment(entity, &key)
		})
		.await
	}

	pub async fn with_async<F, O>(entity: AsyncEntity, func: F) -> O
	where
		F: 'static + Send + Sync + Clone + FnOnce(&mut RouteQuery, Entity) -> O,
		O: 'static + Send + Sync,
	{
		let id = entity.id();
		entity
			.world()
			.run_system_once_with(
				|In((func, id)): In<(F, Entity)>, mut query: RouteQuery| {
					func.clone()(&mut query, id)
				},
				(func, id),
			)
			.await
			.unwrap()
	}

	/// Get the endpoint tree for the given action.
	///
	/// The tree must be present on an ancestor entity, which is done
	/// automatically when using [`router_exchange`] to spawn the router.
	/// This method traverses [`ChildOf`], [`TemplateOf`], and [`ActionOf`]
	/// relationships to find the tree, handling cases where RSX templates
	/// and flow_exchange create separate entity hierarchies.
	///
	/// # Errors
	///
	/// Returns an error if the [`EndpointTree`] is not found on any ancestor.
	/// Use [`router_exchange`] instead of [`flow_exchange`] to ensure the tree
	/// is constructed on spawn.
	pub fn endpoint_tree(&self, action: Entity) -> Result<EndpointTree> {
		// Traverse ancestors using ChildOf, TemplateOf, and ActionOf relationships
		// This handles:
		// 1. RSX template hierarchies (TemplateOf)
		// 2. Flow exchange action hierarchies (ActionOf -> agent -> ChildOf)
		// 3. Direct parent chains (ChildOf)
		let mut current = action;
		let mut depth = 0;
		const MAX_DEPTH: usize = 100;

		loop {
			// Check if current entity has EndpointTree
			if let Ok(tree) = self.endpoint_trees.get(current) {
				return tree.clone().xok();
			}

			// Try different relationship types in order of preference
			if let Ok(template_of) = self.templates.get(current) {
				// Follow TemplateOf for RSX template chains
				current = template_of.get();
			} else if let Ok(parent) = self.parents.get(current) {
				// Follow ChildOf for direct parent chains
				current = parent.get();
			} else if let Ok(action_of) = self.action_ofs.get(current) {
				// Follow ActionOf to get to the agent, then continue from there
				// This handles the case where flow_exchange's action entity
				// has ActionOf but no ChildOf
				current = action_of.get();
			} else {
				// No more ancestors
				break;
			}

			depth += 1;
			if depth > MAX_DEPTH {
				break;
			}
		}

		bevybail!(
			"EndpointTree not found on any ancestor entity. \
			Use `router_exchange` instead of `flow_exchange` to ensure \
			the endpoint tree is constructed on spawn."
		)
	}
}


#[derive(SystemParam)]
pub struct RouteParamQuery<'w, 's, T: Clone + Component> {
	pub agents: AgentQuery<'w, 's>,
	pub params: ParamQuery<'w, 's, T>,
}

impl<T: Clone + Component> RouteParamQuery<'_, '_, T> {
	pub fn get(&mut self, action: Entity) -> Result<T>
	where
		T: Sized + Clone + FromReflect + Typed + Component,
	{
		let agent = self.agents.entity(action);
		self.params.get(agent)
	}
}
