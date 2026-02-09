use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

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


pub fn interface_tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: DeserializeOwned,
	H::Out: Serialize,
{
	(
		ToolMeta::of::<H::In, H::Out>(),
		exchange_tool_handler::<H::In, H::Out>(),
		handler.into_handler(),
	)
}


/// Creates a tool that:
/// 1. accepts a [`Request`]
/// 2. deserializes it to `In`
/// 3. Makes an inner `In`/ `Out` tool call on the entity
/// then internally deserializes it to `In`
fn exchange_tool_handler<In: DeserializeOwned, Out: Serialize>() -> impl Bundle
{
	OnSpawn::observe(
		|ev: On<ToolIn<Request, Response>>,
		 mut commands: Commands,
		 mut async_commands: AsyncCommands|
		 -> Result {
			let (inner_send, inner_recv) = async_channel::bounded::<Out>(1);
			let inner_out_handler = ToolOutHandler::channel(inner_send);
			let val = ev.take_payload()?;
			let entity = ev.event_target();

			async_commands.run(async move |world| -> Result {
				// let input:I
				let val = inner_recv.recv().await?;
				let res: Body = todo!("deserialize val");

				// TODO serialize val
				world.entity(entity).trigger(|entity| {
					ToolIn::new(entity, input, ev.out_handler)
				});
				Ok(())
			});
			Ok(())
		},
	)

	// todo!(
	// 	"same tool as In accepts request, need to perform unchecked tool call?"
	// );
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
