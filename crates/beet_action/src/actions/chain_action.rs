use crate::prelude::*;
use beet_core::prelude::*;

/// Allows chaining two [`Action`] implementations, feeding the
/// output of the first into the input of the second.
///
/// Both handlers are fused into a single [`Action`] whose input
/// type matches the first handler and whose output type matches the
/// second. The intermediate value is converted via [`From`].
pub trait IntoChainAction<In, Out, M>
where
	Self: 'static + Sized + IntoAction<M, In = In, Out = Out>,
	Out: 'static,
{
	/// Chain `self` to `other`, producing a combined handler.
	fn chain<T2, M2>(self, other: T2) -> Action<In, T2::Out>
	where
		T2: 'static + IntoAction<M2>,
		T2::In: 'static + From<Out>,
	{
		let handler1 = self.into_action();
		let handler2 = other.into_action();

		Action::new(
			TypeMeta::of::<(Self, T2)>(),
			move |ActionCall {
			          commands,
			          caller,
			          input: in_a,
			          out_handler,
			      }: ActionCall<In, T2::Out>| {
				let handler2 = handler2.clone();
				handler1.call(ActionCall {
					commands,
					caller,
					input: in_a,
					out_handler: OutHandler::new(
						move |commands, result: Result<Out>| match result {
							Ok(out_a) => {
								handler2.call(ActionCall::<T2::In, T2::Out> {
									commands,
									caller,
									input: out_a.into(),
									out_handler,
								})
							}
							Err(err) => out_handler.call(commands, Err(err)),
						},
					),
				})
			},
		)
	}
}


impl<In, Out, M, T> IntoChainAction<In, Out, M> for T
where
	T: 'static + IntoAction<M, In = In, Out = Out>,
	Out: 'static,
{
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[action(pure)]
	fn add((a, b): (i32, i32)) -> i32 { a + b }
	#[action(pure)]
	fn negate(val: i32) -> i32 { -val }
	#[action(pure)]
	fn multiply(val: i32) -> i32 { val * val }
	#[action(pure)]
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
