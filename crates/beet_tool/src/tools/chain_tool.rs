use crate::prelude::*;

/// Allows chaining two [`Tool`] implementations, feeding the
/// output of the first into the input of the second.
///
/// Both handlers are fused into a single [`Tool`] whose input
/// type matches the first handler and whose output type matches the
/// second. The intermediate value is converted via [`From`].
pub trait IntoChainTool<In, Out, M>
where
	Self: 'static + Sized + IntoTool<M, In = In, Out = Out>,
	Out: 'static,
{
	/// Chain `self` to `other`, producing a combined handler.
	fn chain<T2, M2>(self, other: T2) -> Tool<In, T2::Out>
	where
		T2: 'static + IntoTool<M2>,
		T2::In: 'static + From<Out>,
	{
		let handler1 = self.into_tool();
		let handler2 = other.into_tool();

		Tool::new(
			TypeMeta::of::<(Self, T2)>(),
			move |ToolCall {
			          commands,
			          tool,
			          input: in_a,
			          out_handler,
			      }: ToolCall<In, T2::Out>| {
				let handler2 = handler2.clone();
				handler1.call(ToolCall {
					commands,
					tool,
					input: in_a,
					out_handler: OutHandler::new(
						move |commands, out_a: Out| {
							handler2.call(ToolCall::<T2::In, T2::Out> {
								commands,
								tool,
								input: out_a.into(),
								out_handler,
							})
						},
					),
				})
			},
		)
	}
}


impl<In, Out, M, T> IntoChainTool<In, Out, M> for T
where
	T: 'static + IntoTool<M, In = In, Out = Out>,
	Out: 'static,
{
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[tool]
	fn add(a: i32, b: i32) -> i32 { a + b }
	#[tool]
	fn negate(val: i32) -> i32 { -val }
	#[tool]
	fn multiply(val: i32) -> i32 { val * val }
	#[tool]
	fn to_string(val: i32) -> String { val.to_string() }

	#[test]
	fn two() {
		AsyncPlugin::world()
			.spawn(add.chain(negate))
			.call_blocking::<(i32, i32), i32>((5, 2))
			.unwrap()
			.xpect_eq(-7);
	}
	#[test]
	fn three() {
		AsyncPlugin::world()
			.spawn(add.chain(multiply).chain(negate))
			.call_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(-64);
	}

	#[test]
	fn type_conversion() {
		AsyncPlugin::world()
			.spawn(add.chain(to_string))
			.call_blocking::<(i32, i32), String>((3, 4))
			.unwrap()
			.xpect_eq("7".to_string());
	}

	#[test]
	fn with_closure() {
		AsyncPlugin::world()
			.spawn(add.chain(|val: i32| val * 2))
			.call_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(14);
	}

	#[test]
	fn called_multiple_times() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(add.chain(negate)).id();
		world
			.entity_mut(entity)
			.call_blocking::<(i32, i32), i32>((1, 2))
			.unwrap()
			.xpect_eq(-3);
		world
			.entity_mut(entity)
			.call_blocking::<(i32, i32), i32>((10, 20))
			.unwrap()
			.xpect_eq(-30);
	}

	#[test]
	fn identity() {
		AsyncPlugin::world()
			.spawn(add.chain(|val: i32| val))
			.call_blocking::<(i32, i32), i32>((5, 5))
			.unwrap()
			.xpect_eq(10);
	}
}
