use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Interface {
	/// The card currently being accessed by the interface,
	/// defaulting to the root.
	current_card: Entity,
}

impl Interface {
	pub fn new(card: Entity) -> Self { Self { current_card: card } }

	/// Create a new [`Interface`] pointing to the entity it was inserted on.
	pub fn new_this() -> impl Bundle {
		OnSpawn::new(|entity| {
			let id = entity.id();
			entity.insert(Interface::new(id));
		})
	}
}


/// Create an interface from a handler, inserting an [`Interface`]
/// pointing to itself as the current card.
pub fn interface<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M, In = Request, Out = Response>,
{
	(Interface::new_this(), tool(handler))
}



pub fn markdown_interface() -> impl Bundle {
	interface(
		|In(cx): In<ToolContext<Request>>,
		 interfaces: Query<&Interface>,
		 card_query: CardQuery|
		 -> Result<Response> {
			let interface = interfaces.get(cx.tool)?;
			let tree = card_query.tool_tree(interface.current_card)?;

			if cx.has_param("help") {
				// TODO filter by partial matches, ie tree.filter_path(req.path());
				// any path pattern match for those routes that doesnt error
				// and then print help for all matches
			} else {
				let tool = tree.find_exact(req.path())?;
				// either run the tool or return the markdown as a string.
			}


			Ok(())
		},
	)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn my_interface() -> impl Bundle {
		(
			Interface::new_this(),
			tool(
				|req: In<ToolContext<Request>>,
				 trees: Query<&ToolTree>,
				 interfaces: Query<&Interface>|
				 -> Result<ToolTree> {
					let _interface = interfaces.get(req.tool)?;
					let tree = trees.get(req.tool)?;
					// let tool = tree.find(req.path())?;

					Ok(tree.clone())
				},
			),
			children![(
				PathPartial::new("add"),
				tool(|(a, b): (u32, u32)| a + b)
			)],
		)
	}


	#[test]
	fn works() {
		ToolPlugin::world()
			.spawn(my_interface())
			.call_blocking::<_, ToolTree>(Request::get("foo"))
			.unwrap()
			// 0 is the root, with no tool.
			.flatten()[1]
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}
}
