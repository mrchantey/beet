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
/// BevyRuntime::with(|app|app.run());
/// ```
pub struct BevyRuntime;

impl BevyRuntime {
	pub fn with<R>(mut func: impl FnMut(&mut App) -> R) -> R {
		CURRENT_APP.with(|current| {
			let mut current = current.borrow_mut();
			func(&mut current)
		})
	}
	/// Used by [`RstmlToRsx`] when it encounters a block node:
	/// ```
	/// # use beet_rsx::prelude::*;
	/// let block = "hello";
	/// let node = rsx!{<div>{block}</div>};
	/// ```
	pub fn parse_block_node<M>(
		tracker: RustyTracker,
		block: impl 'static + Clone + IntoRsx<M>,
	) -> RsxNode {
		RsxNode::Block(RsxBlock {
			initial: Box::new(block.clone().into_rsx()),
			effect: Effect::new(
				Box::new(move |_loc: DomLocation| {
					todo!();
					// effect(move || {
					// 	let block = block.clone();
					// 	CurrentHydrator::with(move |hydrator| {
					// 		let node = block.clone().into_rsx();
					// 		hydrator.update_rsx_node(node, loc).unwrap()
					// 	});
					// });
					// Ok(())
				}),
				tracker,
			),
		})
	}
	/// Used by [`RstmlToRsx`] when it encounters an attribute block:
	/// ```
	/// # use beet_rsx::prelude::*;
	/// let value = || vec![RsxAttribute::Key{key:"foo".to_string()}];
	/// let node = rsx!{<el {value}/>};
	/// ```
	pub fn parse_attribute_block(
		tracker: RustyTracker,
		mut block: impl 'static + FnMut() -> Vec<RsxAttribute>,
	) -> RsxAttribute {
		RsxAttribute::Block {
			initial: block(),
			effect: Effect::new(
				Box::new(|_loc| {
					todo!();
				}),
				tracker,
			),
		}
	}


	/// Used by [`RstmlToRsx`] when it encounters an attribute with a block value:
	/// ```
	/// # use beet_rsx::prelude::*;
	/// let value = 3;
	/// let node = rsx!{<el key={value}/>};
	/// ```
	pub fn parse_attribute_value<T: 'static + Clone + PartialReflect>(
		key: &'static str,
		tracker: RustyTracker,
		value: T,
	) -> RsxAttribute {
		let initial = Self::with(|app| {
			// let type_id = TypeId::of::<T>();
			let registry = app.world().resource::<AppTypeRegistry>();
			let registry = registry.read();
			let reflect_serializer =
				TypedReflectSerializer::new(&value, &registry);
			ron::to_string(&reflect_serializer).unwrap()
		});

		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial,
			effect: Effect::new(
				Box::new(|_loc| {
					todo!();
				}),
				tracker,
			),
		}
	}
}



#[cfg(test)]
mod test {
	// use crate::prelude::*;
	use super::BevyRuntime;
	use crate::rsx::RsxAttribute;
	use crate::rsx::RustyTracker;
	// use bevy::ecs::reflect::AppTypeRegistry;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		BevyRuntime::with(|app| {
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
}
