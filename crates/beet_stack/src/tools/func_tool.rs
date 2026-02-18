use crate::prelude::*;

/// Blanket [`IntoToolHandler2`] impl for closures that accept an argument
/// convertible from [`ToolContext`].
impl<F, In, Arg, ArgM, Out> IntoToolHandler2<(In, Arg, ArgM, Out)> for F
where
	F: 'static + Send + Sync + FnMut(Arg) -> Out,
	Arg: FromToolContext<In, ArgM>,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(mut self) -> ToolHandler<Self::In, Self::Out> {
		ToolHandler::new(
			move |ToolCall {
			          commands,
			          tool,
			          input,
			          out_handler,
			      }| {
				let arg = Arg::from_tool_context(ToolContext { tool, input });
				let out = self(arg);
				out_handler.call(commands, out)
			},
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn pure_add() {
		AsyncPlugin::world()
			.spawn(tool2(|(a, b): (i32, i32)| a + b))
			.call2_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(8);
	}

	#[test]
	fn returns_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(tool2(|cx: ToolContext<()>| -> Entity { cx.tool }))
			.id();
		world
			.entity_mut(entity)
			.call2_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[test]
	fn captures_mutable() {
		let mut val = 0;
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(tool2(move |_cx: ToolContext| -> i32 {
				val += 1;
				val
			}))
			.id();
		world
			.entity_mut(entity)
			.call2_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(1);
		world
			.entity_mut(entity)
			.call2_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(2);
	}
}
