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

	#[beet_core::test]
	async fn two() {
		AsyncPlugin::world()
			.spawn(add.chain(negate))
			.call::<(i32, i32), i32>((5, 2))
			.await
			.unwrap()
			.xpect_eq(-7);
	}
	#[beet_core::test]
	async fn three() {
		AsyncPlugin::world()
			.spawn(add.chain(multiply).chain(negate))
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(-64);
	}

	#[beet_core::test]
	async fn type_conversion() {
		AsyncPlugin::world()
			.spawn(add.chain(to_string))
			.call::<(i32, i32), String>((3, 4))
			.await
			.unwrap()
			.xpect_eq("7".to_string());
	}

	#[beet_core::test]
	async fn with_closure() {
		AsyncPlugin::world()
			.spawn(add.chain(|val: i32| val * 2))
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(14);
	}

	#[beet_core::test]
	async fn called_multiple_times() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(add.chain(negate)).id();
		world
			.entity_mut(entity)
			.call::<(i32, i32), i32>((1, 2))
			.await
			.unwrap()
			.xpect_eq(-3);
		world
			.entity_mut(entity)
			.call::<(i32, i32), i32>((10, 20))
			.await
			.unwrap()
			.xpect_eq(-30);
	}

	#[beet_core::test]
	async fn identity() {
		AsyncPlugin::world()
			.spawn(add.chain(|val: i32| val))
			.call::<(i32, i32), i32>((5, 5))
			.await
			.unwrap()
			.xpect_eq(10);
	}
}
