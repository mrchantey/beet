use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::serde::TypedReflectSerializer;
use std::cell::RefCell;

thread_local! {
	static CURRENT_APP: RefCell<App> = RefCell::new(App::new());
}

/// A runtime for working with bevy. The pattern of `static CURRENT_APP` is
/// intende for authoring only, once the authoring step is done this can be
/// taken or ran:
/// ```
/// # use beet_rsx::prelude::*;
/// BevyRuntime::with_mut(|app|app.run());
/// ```
pub struct BevyRuntime;

impl BevyRuntime {
	pub fn reset() { Self::with_mut(|app| *app = App::new()); }
	/// Take the bevy app, replacing it with default.
	pub fn take() -> App { Self::with_mut(|app| std::mem::take(app)) }
	pub fn with_mut<R>(func: impl FnOnce(&mut App) -> R) -> R {
		CURRENT_APP.with(|current| {
			let mut current = current.borrow_mut();
			func(&mut current)
		})
	}
	/// Used by [`RstmlToRsx`] when it encounters a block node:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let block = "hello";
	/// let node = rsx!{<div>{block}</div>};
	/// ```
	pub fn parse_block_node<M1, M2>(
		tracker: RustyTracker,
		block: impl Clone + IntoRsxNode<M1> + SignalOrComponent<M2>,
	) -> RsxNode {
		RsxNode::Block(RsxBlock {
			initial: Box::new(block.clone().into_node()),
			effect: Effect::new(block.into_node_block_effect(), tracker),
			meta: RsxNodeMeta::default(),
		})
	}
	/// Used by [`RstmlToRsx`] when it encounters an attribute block:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// #[derive(IntoBlockAttribute)]
	/// struct Foo;
	/// let node = rsx!{<el {Foo}/>};
	/// ```
	#[allow(unused)]
	pub fn parse_attribute_block<M, T: SignalOrRon<M>>(
		tracker: RustyTracker,
		mut block: T,
	) -> RsxAttribute {
		todo!()
	}


	/// Used by [`RstmlToRsx`] when it encounters an attribute with a block value:
	/// ```
	/// # use beet_rsx::as_beet::*;
	/// let value = 3;
	/// let node = rsx!{<el key={value}/>};
	/// ```
	pub fn parse_attribute_value<M, T: SignalOrRon<M>>(
		field_path: &'static str,
		tracker: RustyTracker,
		mut value: T,
	) -> RsxAttribute {
		let initial = value.into_ron_str();

		RsxAttribute::BlockValue {
			key: field_path.to_string(),
			initial,
			effect: Effect::new(
				value.into_attribute_value_effect(field_path.to_string()),
				tracker,
			),
		}
	}

	pub fn serialize(val: &impl PartialReflect) -> ron::Result<String> {
		Self::with_mut(|app| {
			// let type_id = TypeId::of::<T>();
			let registry = app.world().resource::<AppTypeRegistry>();
			let registry = registry.read();
			let reflect_serializer =
				TypedReflectSerializer::new(val, &registry);
			ron::to_string(&reflect_serializer)
		})
	}
}



#[cfg(test)]
mod test {
	use super::BevyRuntime;
	use crate::as_beet::*;
	// use bevy::ecs::reflect::AppTypeRegistry;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn initial() {
		BevyRuntime::reset();
		BevyRuntime::with_mut(|app| {
			// app.init_resource::<AppTypeRegistry>();
			app.register_type::<Vec3>();
		});

		let attr = BevyRuntime::parse_attribute_value(
			"Transform.translation",
			RustyTracker::new(0, 0),
			Vec3::new(0., 1., 2.),
		);

		let RsxAttribute::BlockValue {
			key,
			initial,
			effect: _,
		} = attr
		else {
			panic!()
		};
		expect(&key).to_be("Transform.translation");
		expect(&initial).to_be("(0.0,1.0,2.0)");
	}

	#[test]
	#[cfg(feature = "bevy_default")]
	fn block_node() {
		BevyRuntime::reset();
		let (get, set) = BevySignal::signal(1);

		let node = rsx! { <entity runtime:bevy>{get}</entity> };
		RsxToBevy::spawn(node).unwrap();
		set(3);

		let mut app = BevyRuntime::with_mut(|app| std::mem::take(app));
		// flush signals
		app.update();
		let world = app.world_mut();
		let mut query = world.query::<&Text>();
		let text = query.iter(world).next().unwrap();
		expect(&text.0).to_be("3");
	}

	#[test]
	fn attr_value() {
		BevyRuntime::reset();
		BevyRuntime::with_mut(|app| {
			app.register_type::<Transform>();
		});

		let (get, set) = BevySignal::signal(Vec3::new(0., 1., 2.));
		let rsx = rsx! { <entity runtime:bevy Transform.translation=get /> };
		RsxToBevy::spawn(rsx).unwrap();
		set(Vec3::new(3., 4., 5.));

		let mut app = BevyRuntime::with_mut(|app| std::mem::take(app));
		// flush signals
		app.update();
		let world = app.world_mut();
		let mut query = world.query::<&Transform>();
		let transform = query.iter(world).next().unwrap();
		expect(transform.translation).to_be(Vec3::new(3., 4., 5.));
	}
}
