use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::Reflectable;

pub trait SignalPayload: 'static + Send + Sync + Clone + Reflectable {}
impl<T: 'static + Send + Sync + Clone + Reflectable> SignalPayload for T {}



/// An example implementation of bevy signals. The machinery is quite straightforward,
/// its just an observer, but the way that the scene gets updated by changes is
/// where things get more interesting, see [`SignalGetter`]
#[derive(Event, Clone)]
pub struct BevySignal<T> {
	// pub idx: RsxIdx,
	pub value: T,
}

impl<T: SignalPayload> BevySignal<T> {
	pub fn signal(initial: T) -> (SignalGetter<T>, impl Fn(T) -> ()) {
		let entity = BevyRuntime::with(move |app| {
			// app.init_resource::<AppTypeRegistry>();
			app.register_type::<T>();

			let sig = BevySignal {
				value: initial.clone(),
			};

			let entity = app
				.world_mut()
				.spawn(sig)
				.observe(
					|ev: Trigger<BevySignal<T>>,
					 mut commands: Commands,
					 mut query: Query<&mut BevySignal<T>>| {
						let new_val = ev.event().clone();
						if let Ok(mut val) = query.get_mut(ev.entity()) {
							*val = new_val;
						} else {
							commands.entity(ev.entity()).insert(new_val);
						}
					},
				)
				.id();
			app.world_mut().flush();
			entity
		});


		let set = move |val: T| {
			BevyRuntime::with(move |app| {
				app.world_mut()
					.entity_mut(entity)
					.trigger(BevySignal { value: val.clone() });
				app.world_mut().flush();
			})
		};
		(SignalGetter::new(entity), set)
	}
}

/// The 'get' function of a signal. These can be applied to
/// attribute values or node blocks.
#[derive(Debug, Copy, Clone)]
pub struct SignalGetter<T> {
	pub entity: Entity,
	phantom: std::marker::PhantomData<T>,
}
impl<T: SignalPayload> SignalGetter<T> {
	pub fn new(entity: Entity) -> Self {
		Self {
			entity,
			phantom: std::marker::PhantomData,
		}
	}

	pub fn get(&self) -> T {
		BevyRuntime::with(|app| {
			app.world()
				.get::<BevySignal<T>>(self.entity)
				.unwrap()
				.value
				.clone()
		})
	}
}

impl<T: SignalPayload> FnOnce<()> for SignalGetter<T> {
	type Output = T;
	extern "rust-call" fn call_once(self, _args: ()) -> T { self.get() }
}

impl<T: SignalPayload> FnMut<()> for SignalGetter<T> {
	extern "rust-call" fn call_mut(&mut self, _args: ()) -> T { self.get() }
}

impl<T: SignalPayload> Fn<()> for SignalGetter<T> {
	extern "rust-call" fn call(&self, _args: ()) -> T { self.get() }
}


/// Trait for signals, literals and blocks that can be converted to
/// bevy entities
pub trait SignalOrComponent<M>: 'static + Send + Sync + Clone {
	type Component: bevy::prelude::Component + Reflectable;
	/// Serialize using ron
	fn into_component(self) -> Self::Component;

	fn into_node_block_effect(self) -> RegisterEffect;
}

pub struct ToStringMarker;
impl<T: 'static + Send + Sync + Clone + ToString>
	SignalOrComponent<(T, ToStringMarker)> for T
{
	type Component = Text;
	fn into_component(self) -> Self::Component { Text(self.to_string()) }

	fn into_node_block_effect(self) -> RegisterEffect { Box::new(|_| Ok(())) }
}
pub struct GetterMarker;
impl<T: Reflectable + SignalOrComponent<M>, M>
	SignalOrComponent<(M, GetterMarker)> for SignalGetter<T>
{
	type Component = T::Component;
	fn into_component(self) -> Self::Component { self.get().into_component() }

	fn into_node_block_effect(self) -> RegisterEffect {
		Box::new(move |loc: DomLocation| {
			BevyRuntime::with(move |app| {
				app.world_mut().entity_mut(self.entity).observe(
					move |ev: Trigger<BevySignal<T>>,
					      // idx_query: Query<&BevyRsxIdx>,
					      mut query: Query<(
						&BevyRsxIdx,
						&mut T::Component,
					)>| {
						for (idx, mut component) in query.iter_mut() {
							// isnt working as expected because register_effect provides
							// the location of the block, not the initial which is +1
							println!("{:?} {:?}", idx, loc);
							if **idx == loc.rsx_idx {
								*component =
									ev.event().value.clone().into_component();
							}
						}
					},
				);
				app.world_mut().flush();
			});
			Ok(())
		})
	}
}





/// Trait for signals, literals and blocks that can be converted to
/// bevy attribute values
pub trait SignalOrRon<M>: 'static + Send + Sync + Clone {
	type Inner: SignalPayload;
	/// Serialize using ron
	fn into_ron_str(&self) -> String;
	fn into_attribute_value_effect(self, field_path: String) -> RegisterEffect;
}

pub struct PayloadIntoBevyAttributeValue;
impl<T: SignalPayload> SignalOrRon<(T, PayloadIntoBevyAttributeValue)> for T {
	type Inner = T;
	fn into_ron_str(&self) -> String { BevyRuntime::serialize(self).unwrap() }
	fn into_attribute_value_effect(
		self,
		_field_path: String,
	) -> RegisterEffect {
		Box::new(|_| Ok(()))
	}
}
pub struct GetterIntoRsxAttribute;
impl<T: SignalPayload + SignalOrRon<M>, M>
	SignalOrRon<(M, GetterIntoRsxAttribute)> for SignalGetter<T>
{
	type Inner = T::Inner;
	fn into_ron_str(&self) -> String { self.get().into_ron_str() }

	/// A registration function that will update the attribute value,
	/// which can either be a bevy Component or a field of that component,
	/// with the value of the signal when it changes.
	fn into_attribute_value_effect(self, field_path: String) -> RegisterEffect {
		Box::new(move |loc| {
			BevyRuntime::with(move |app| {
				app.world_mut().entity_mut(self.entity).observe(
					move |ev: Trigger<BevySignal<T>>,
					      registry: Res<AppTypeRegistry>,
					      mut query: Query<EntityMut, With<BevyRsxIdx>>| {
						let entity = BevyRsxIdx::find_mut(&mut query, loc)
							.expect(&expect_rsx_element::to_be_at_location(
								&loc,
							));
						let registry = registry.read();
						ReflectUtils::apply_at_path(
							&registry,
							entity,
							&field_path,
							ev.event().value.clone(),
						)
						.unwrap();
					},
				);
				app.world_mut().flush();
			});
			Ok(())
		})
	}
}


#[cfg(test)]
mod test {
	use super::BevyRuntime;
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn signal() {
		let (get, set) = BevySignal::signal(7);
		expect(get()).to_be(7);
		set(8);
		BevyRuntime::with(|a| a.world_mut().flush());
		expect(get()).to_be(8);
	}
}
