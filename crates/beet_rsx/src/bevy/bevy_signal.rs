use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::serde::TypedReflectSerializer;
use bevy::reflect::Reflectable;

pub trait SignalPayload: 'static + Send + Sync + Clone + Reflectable {}
impl<T: 'static + Send + Sync + Clone + Reflectable> SignalPayload for T {}

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




///
///
///
pub trait IntoBevyAttrVal<M>: 'static + Send + Sync + Clone {
	type Inner: SignalPayload;
	fn into_bevy_val(&self) -> String;
	/// if this is a signal, return the entity containing
	/// the [`BevySignal`] component
	fn signal_entity(&self) -> Option<Entity> { None }
}

pub struct PayloadIntoBevyAttributeValue;
impl<T: SignalPayload> IntoBevyAttrVal<(T, PayloadIntoBevyAttributeValue)>
	for T
{
	type Inner = T;
	fn into_bevy_val(&self) -> String {
		BevyRuntime::with(|app| {
			// let type_id = TypeId::of::<T>();
			let registry = app.world().resource::<AppTypeRegistry>();
			let registry = registry.read();
			let reflect_serializer =
				TypedReflectSerializer::new(self, &registry);
			ron::to_string(&reflect_serializer).unwrap()
		})
	}
}
pub struct GetterIntoRsxAttribute;
impl<T: SignalPayload + IntoBevyAttrVal<M>, M>
	IntoBevyAttrVal<(M, GetterIntoRsxAttribute)> for SignalGetter<T>
{
	type Inner = T;
	fn into_bevy_val(&self) -> String { self.get().into_bevy_val() }
	fn signal_entity(&self) -> Option<Entity> { Some(self.entity) }
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
