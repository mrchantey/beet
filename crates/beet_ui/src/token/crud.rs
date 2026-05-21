use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::Typed;

/// References a [`Token`] from an entity, so [`Action`]s like [`Push`] and
/// [`RemoveAt`] can resolve which token to act on.
#[derive(Debug, Clone, Component, Reflect, Deref)]
#[reflect(Component)]
pub struct TokenRef(pub Token);

impl TokenRef {
	pub fn new(token: impl Into<Token>) -> Self { Self(token.into()) }
}

/// Replace the value of the referenced [`Token`] with the input.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Set<T>(
	cx: In<ActionContext<T>>,
	mut tokens: TokenQuery,
	refs: Query<&TokenRef>,
) -> Result
where
	T: 'static + Send + Sync + Typed + Serialize,
{
	let token = refs.get(cx.id())?;
	tokens.set(token, cx.input)
}

/// Append the input to the list-typed [`Token`] referenced by this entity.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn Push<T>(
	cx: In<ActionContext<T>>,
	mut tokens: TokenQuery,
	refs: Query<&TokenRef>,
) -> Result
where
	T: 'static + Send + Sync + Typed + Serialize,
{
	let token = refs.get(cx.id())?;
	tokens.push(token, cx.input)
}

/// Insert a value at a given index of the referenced list-typed [`Token`].
/// The input is `(index, value)`. Out-of-range indices are clamped to the
/// list length.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn InsertAt<T>(
	cx: In<ActionContext<(usize, T)>>,
	mut tokens: TokenQuery,
	refs: Query<&TokenRef>,
) -> Result
where
	T: 'static + Send + Sync + Typed + Serialize + GetTypeRegistration,
{
	let token = refs.get(cx.id())?;
	let (index, value) = cx.take();
	tokens.insert(token, index, value)
}

/// Remove the value at the given index of the referenced list-typed
/// [`Token`], returning the removed [`Value`] if present.
#[action]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub fn RemoveAt(
	cx: In<ActionContext<usize>>,
	mut tokens: TokenQuery,
	refs: Query<&TokenRef>,
) -> Result<Option<Value>> {
	let token = refs.get(cx.id())?;
	tokens.remove_at(token, cx.input)
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	fn list_token() -> TokenDefinition<Vec<i32>> {
		TokenDefinition::inline(Vec::new())
	}

	#[beet_core::test]
	async fn push_appends() {
		let mut world = AsyncPlugin::world();
		let token = list_token();
		let token_ref = TokenRef::new(&token);
		let host = world.spawn(token.into_bundle()).id();
		let actor = world.spawn((token_ref, Push::<i32>::default())).id();

		world.entity_mut(actor).call::<i32, ()>(7).await.unwrap();
		world.entity_mut(actor).call::<i32, ()>(8).await.unwrap();

		// Value::List uses Int internally for i32 inputs via serde
		world
			.entity(host)
			.get::<Value>()
			.unwrap()
			.xpect_eq(val!([7i64, 8i64]));
		// silence warning that `host` is unused
		let _ = host;
	}

	#[beet_core::test]
	async fn insert_and_remove() {
		let mut world = AsyncPlugin::world();
		let token = list_token();
		let token_ref = TokenRef::new(&token);
		let host = world.spawn(token.into_bundle()).id();
		let actor = world
			.spawn((
				token_ref,
				Push::<i32>::default(),
				InsertAt::<i32>::default(),
				RemoveAt,
			))
			.id();

		for v in [1i32, 2, 3] {
			world.entity_mut(actor).call::<i32, ()>(v).await.unwrap();
		}
		// list is now [1, 2, 3]
		world
			.entity_mut(actor)
			.call::<(usize, i32), ()>((1, 99))
			.await
			.unwrap();
		// list is now [1, 99, 2, 3]
		world
			.entity_mut(actor)
			.call::<usize, Option<Value>>(0)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(val!(1i64));

		world
			.entity(host)
			.get::<Value>()
			.unwrap()
			.xpect_eq(val!([99i64, 2i64, 3i64]));
	}

	#[beet_core::test]
	async fn set_replaces() {
		let mut world = AsyncPlugin::world();
		let count = TokenDefinition::inline(0i32);
		let token_ref = TokenRef::new(&count);
		let host = world.spawn(count.into_bundle()).id();
		let actor = world
			.spawn((token_ref, Set::<i32>::default()))
			.id();

		world.entity_mut(actor).call::<i32, ()>(42).await.unwrap();

		world
			.entity(host)
			.get::<Value>()
			.unwrap()
			.xpect_eq(Value::Int(42));
	}
}
