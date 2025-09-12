use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use std::sync::Arc;

/// Mark an entity as requiring a [`DomBinding`], often added to nodes
/// with a [`SignalEffect`], and also their parent [`ElementNode`]
/// in the case of a [`FragmentNode`]
#[derive(Default, Component, Clone)]
pub struct RequiresDomBinding;

/// Basic primitives required for downstream crates to implement reactivity,
/// This type should be marked [`Changed`] when its associated signal changes,
/// which is important for cases like bundle signals where the 'whole entity' should
/// be watched for changes instead of a single component.
/// see beet_rsx::reactivity::propagate_signal_effect.rs
// This is implemented here due to orphan rule
#[derive(Component, Clone)]
#[require(RequiresDomBinding)]
pub struct SignalEffect {
	system_id: SystemId,
	/// A function that calls the getter, so calling it inside a
	/// new [`effect`] will subscribe to its changes.
	effect_subscriber: Arc<dyn 'static + Send + Sync + Fn()>,
}

impl SignalEffect {
	pub fn new<Func, Out>(func: Func, system_id: SystemId) -> Self
	where
		Func: 'static + Send + Sync + Clone + FnOnce() -> Out,
	{
		let effect_subscriber = Arc::new(move || {
			let _ = func.clone()();
		});
		SignalEffect {
			system_id,
			effect_subscriber,
		}
	}
	pub fn system_id(&self) -> SystemId { self.system_id }
	pub fn effect_subscriber(&self) -> Arc<dyn 'static + Send + Sync + Fn()> {
		self.effect_subscriber.clone()
	}
}


pub struct PrimitiveGetterIntoBundle;

/// we dont want to blanket impl ToString because collision
/// with Bundle impl, feel free to open pr to add more as required.
trait Primitive<M> {
	fn primitive_string(&self) -> String;
}
macro_rules! impl_primitive {
    ($($ty:ty),*) => {
        $(
            impl Primitive<Self> for $ty {
                fn primitive_string(&self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}

impl_primitive!(
	String,
	&'static str,
	std::borrow::Cow<'_, str>,
	bool,
	i8,
	i16,
	i32,
	i64,
	i128,
	isize,
	u8,
	u16,
	u32,
	u64,
	u128,
	usize,
	f32,
	f64
);

impl<T, M> Primitive<(Self, M)> for Option<T>
where
	T: Primitive<M>,
{
	fn primitive_string(&self) -> String {
		match self {
			Some(value) => value.primitive_string(),
			None => String::new(),
		}
	}
}


impl<Func, Out, M> IntoBundle<(Out, M)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce() -> Out,
	Out: 'static + Send + Sync + Clone + Primitive<M>,
{
	fn into_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity| {
			let id = entity.id();
			let this = self.clone();
			let system_id = entity.world_scope(move |world| {
				world.register_system(
					(move |mut query: Query<(
						&mut SignalEffect,
						&mut TextNode,
					)>|
					      -> Result {
						let (mut signal, mut text) = query.get_mut(id)?;
						signal.set_changed();
						text.0 = this.clone()().primitive_string();
						Ok(())
					})
					.pipe(handle_result),
				)
			});

			entity.insert((
				TextNode::new(self.clone()().primitive_string()),
				SignalEffect::new(self, system_id),
			));
		})
	}
}

fn handle_result(result: In<Result>) {
	if let Err(err) = result.0 {
		panic!("Signal Effect Error: {}", err);
	}
}

// pub struct BundleGetterIntoBundle;

/// for bundles and vecs of bundles
trait BundleLike<M1, M2>: IntoBundle<M1> {}
pub struct BundleIntoBundleLike;
impl<T, M> BundleLike<M, BundleIntoBundleLike> for T where
	T: IntoBundle<M> + Bundle
{
}
impl<T, M> BundleLike<M, Self> for Vec<T>
where
	Vec<T>: IntoBundle<M>,
	T: Bundle,
{
}

impl<Func, Out, M1, M2> IntoBundle<(Out, M1, M2)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce() -> Out,
	Out: 'static + Send + Sync + BundleLike<M1, M2>,
{
	fn into_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity| {
			let id = entity.id();
			let this = self.clone();
			let system_id = entity.world_scope(move |world| {
				world.register_system(
					(move |mut commands: Commands,
					       mut query: Query<&mut SignalEffect>|
					      -> Result {
						query.get_mut(id)?.set_changed();
						// remove everything but the SignalEffect and relations
						// we arent doing any fine-grained diffing here, instead
						// we diff the actual dom
						commands
							.entity(id)
							.despawn_related::<Children>()
							.despawn_related::<Attributes>()
							.retain::<(SignalEffect, ChildOf)>()
							.insert(this.clone()().into_bundle());
						Ok(())
					})
					.pipe(handle_result),
				)
			});

			entity.insert((
				self.clone()().into_bundle(),
				SignalEffect::new(self, system_id),
			));
		})
	}
}


// #[cfg(not(feature = "nightly"))]
// impl<T, M> IntoBundle<(Self, T, M)> for beet_core::prelude::Getter<T>
// where
// 	T: 'static + Send + Sync + IntoBundle<M>,
// {
// 	fn into_bundle(self) -> impl Bundle { self.get().into_bundle() }
// }

// more tests in beet_rsx::reactivity::propagate_signal_effect.rs
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	#[cfg(not(feature = "nightly"))]
	#[test]
	fn stable() {
		// panic!();
	}

	#[test]
	fn primitive_getter() {
		let (get, set) = signal("bob".to_string());

		let mut world = World::new();
		let entity = world.spawn(get.into_bundle()).id();
		let assert = |world: &World, name: &str| {
			world
				.entity(entity)
				.get::<TextNode>()
				.unwrap()
				.xpect_eq(TextNode::new(name.to_owned()));
		};

		assert(&world, "bob");
		set("bill".to_string());
		assert(&world, "bob");
		let system = world
			.entity(entity)
			.get::<SignalEffect>()
			.unwrap()
			.system_id();
		world.run_system(system).unwrap();
		assert(&world, "bill");
	}
	#[test]
	fn bundle_getter() {
		let (get, set) = signal(Name::new("bob"));

		let mut world = World::new();
		let entity = world.spawn(get.into_bundle()).id();
		let assert = |world: &World, name: &str| {
			world
				.entity(entity)
				.get::<Name>()
				.unwrap()
				.xpect_eq(Name::new(name.to_owned()));
		};

		assert(&world, "bob");
		set(Name::new("bill"));
		assert(&world, "bob");
		let system = world
			.entity(entity)
			.get::<SignalEffect>()
			.unwrap()
			.system_id();
		world.run_system(system).unwrap();
		assert(&world, "bill");
	}
}
