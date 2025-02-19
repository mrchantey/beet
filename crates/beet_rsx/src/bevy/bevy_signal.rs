use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::Reflectable;
use flume::Receiver;
use flume::Sender;

pub trait SignalPayload: 'static + Send + Sync + Clone + Reflectable {}
impl<T: 'static + Send + Sync + Clone + Reflectable> SignalPayload for T {}

/// Simple channels mechanism for sending signals to entities.
#[derive(Clone, Resource)]
pub struct SignalChannel<T> {
	pub send: Sender<(Entity, T)>,
	pub recv: Receiver<(Entity, T)>,
}
impl<T> Default for SignalChannel<T> {
	fn default() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}
}

impl<T: SignalPayload> SignalChannel<T> {
	pub fn get_or_init(app: &mut App) -> Sender<(Entity, T)> {
		if let Some(channel) = app.world().get_resource::<SignalChannel<T>>() {
			channel.send.clone()
		} else {
			let channel = SignalChannel::default();
			let send = channel.send.clone();
			app.insert_resource(channel);
			app.add_systems(
				Update,
				|res: Res<SignalChannel<T>>, mut commands: Commands| {
					while let Ok((entity, value)) = res.recv.try_recv() {
						commands.entity(entity).trigger(BevySignal::new(value));
					}
				},
			);
			send
		}
	}
}


/// An example implementation of bevy signals. The machinery is quite straightforward,
/// its just an observer, but the way that the scene gets updated by changes is
/// where things get more interesting, see [`SignalGetter`]
#[derive(Event, Clone)]
pub struct BevySignal<T> {
	// pub idx: RsxIdx,
	pub value: T,
}

impl<T: SignalPayload> BevySignal<T> {
	pub fn new(value: T) -> Self { Self { value } }

	pub fn signal(initial: T) -> (SignalGetter<T>, impl Fn(T) -> ()) {
		let initial2 = initial.clone();
		let entity = BevyRuntime::with_mut(move |app| {
			// app.init_resource::<AppTypeRegistry>();
			app.register_type::<T>();

			let entity = app
				.world_mut()
				.spawn(BevySignal::new(initial2))
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


		// we need two channels. one to update bevy, and one to update the getter directly
		let send_to_bevy =
			BevyRuntime::with_mut(|app| SignalChannel::<T>::get_or_init(app));

		let (send_to_getter, recv_from_setter) = flume::unbounded();

		let get = SignalGetter::new(entity, recv_from_setter, initial.clone());
		let set = move |val: T| {
			// if the getter was dropped thats ok
			let err1 = send_to_getter.send(val.clone()).is_err();
			// if the bevy resource was removed thats ok
			let err2 = send_to_bevy.send((entity, val)).is_err();
			// if both error, better warn
			if err1 && err2 {
				eprintln!(
					"Signal setter failed to send to both getter and bevy"
				);
			}
		};

		(get, set)
	}
}

/// The 'get' function of a signal. These can be applied to
/// attribute values or node blocks.
#[derive(Debug, Clone)]
pub struct SignalGetter<T> {
	value: T,
	recv: Receiver<T>,
	pub entity: Entity,
	phantom: std::marker::PhantomData<T>,
}
impl<T: SignalPayload> SignalGetter<T> {
	pub fn new(entity: Entity, recv: Receiver<T>, initial: T) -> Self {
		Self {
			entity,
			recv,
			value: initial,
			phantom: std::marker::PhantomData,
		}
	}

	pub fn get(&mut self) -> T {
		while let Ok(val) = self.recv.try_recv() {
			self.value = val;
		}
		self.value.clone()
	}
}

impl<T: SignalPayload> FnOnce<()> for SignalGetter<T> {
	type Output = T;
	extern "rust-call" fn call_once(mut self, _args: ()) -> T { self.get() }
}

impl<T: SignalPayload> FnMut<()> for SignalGetter<T> {
	extern "rust-call" fn call_mut(&mut self, _args: ()) -> T { self.get() }
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
	fn into_component(mut self) -> Self::Component {
		self.get().into_component()
	}

	fn into_node_block_effect(self) -> RegisterEffect {
		Box::new(move |block_loc: TreeLocation| {
			// register_effect provides the location of the block, not the initial which is +1.
			// i guess its guaranteed to be +1 so we can just increment?
			let inner_idx = *block_loc.tree_idx + 1;
			BevyRuntime::with_mut(move |app| {
				app.world_mut().entity_mut(self.entity).observe(
					move |ev: Trigger<BevySignal<T>>,
					      mut query: Query<(
						&TreeIdx,
						&mut T::Component,
					)>| {
						for (idx, mut component) in query.iter_mut() {
							if **idx == inner_idx {
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
	fn into_ron_str(&mut self) -> String;
	fn into_attribute_value_effect(self, field_path: String) -> RegisterEffect;
}

pub struct PayloadIntoBevyAttributeValue;
impl<T: SignalPayload> SignalOrRon<(T, PayloadIntoBevyAttributeValue)> for T {
	type Inner = T;
	fn into_ron_str(&mut self) -> String {
		BevyRuntime::serialize(self).unwrap()
	}
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
	fn into_ron_str(&mut self) -> String { self.get().into_ron_str() }

	/// A registration function that will update the attribute value,
	/// which can either be a bevy Component or a field of that component,
	/// with the value of the signal when it changes.
	fn into_attribute_value_effect(self, field_path: String) -> RegisterEffect {
		Box::new(move |loc| {
			BevyRuntime::with_mut(move |app| {
				app.world_mut().entity_mut(self.entity).observe(
					move |ev: Trigger<BevySignal<T>>,
					      registry: Res<AppTypeRegistry>,
					      mut query: Query<EntityMut, With<TreeIdx>>| {
						let entity = TreeIdx::find_mut(&mut query, loc).expect(
							&expect_rsx_element::to_be_at_location(&loc),
						);
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
		BevyRuntime::reset();

		let (mut get, set) = BevySignal::signal(7);
		expect(get()).to_be(7);
		set(8);
		// flush signals
		BevyRuntime::with_mut(|a| a.update());
		expect(get()).to_be(8);
	}
}
