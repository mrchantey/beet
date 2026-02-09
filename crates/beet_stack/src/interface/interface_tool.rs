use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Interface {
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


/// Create an interface tool from a handler, enabling both typed `In`/`Out`
/// calls and serialized [`Request`]/[`Response`] calls via [`exchange_tool`].
pub fn interface_tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: serde::de::DeserializeOwned,
	H::Out: serde::Serialize,
{
	exchange_tool(handler)
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
