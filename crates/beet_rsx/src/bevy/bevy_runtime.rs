use crate::prelude::*;
use bevy::prelude::*;
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
	pub fn with<R>(func: impl FnOnce(&mut App) -> R) -> R {
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
	pub fn parse_attribute_value<M, T: IntoBevyAttrVal<M>>(
		field_path: &'static str,
		tracker: RustyTracker,
		value: T,
	) -> RsxAttribute {
		let initial = value.into_bevy_val();

		let register_effect: RegisterEffect = if let Some(sig_entity) =
			value.signal_entity()
		{
			Box::new(move |loc| {
				Self::with(move |app| {
					// let registry = app.world().resource::<AppTypeRegistry>();
					// let registry = registry.read();
					// let registration = registry.get(TypeId::of::<T::Inner>()).expect(
					// 			"Type not registered, please call BevyRuntime::with(|app|app.register_type::<T>())");

					app.world_mut().entity_mut(sig_entity).observe(
						move |ev: Trigger<BevySignal<T::Inner>>,
						      registry: Res<AppTypeRegistry>,
						      mut elements: Query<
							EntityMut,
							With<BevyRsxElement>,
						>| {
							let entity =
								BevyRsxElement::find_mut(&mut elements, loc)
									.expect(
										&expect_rsx_element::to_be_at_location(
											&loc,
										),
									);
							let registry = registry.read();
							ReflectUtils::apply_at_path(
								&registry,
								entity,
								field_path,
								ev.event().value.clone(),
							)
							.unwrap();

							// O(n) search, if we have more than a few hundred entities
							// we should consider a hashmap
						},
					);
					app.world_mut().flush();
				});
				Ok(())
			})
		} else {
			// its a constant
			Box::new(|_loc| Ok(()))
		};


		RsxAttribute::BlockValue {
			key: field_path.to_string(),
			initial,
			effect: Effect::new(register_effect, tracker),
		}
	}
}



#[cfg(test)]
mod test {
	use super::BevyRuntime;
	use crate::prelude::*;
	// use bevy::ecs::reflect::AppTypeRegistry;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn initial() {
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
	#[test]
	fn attr_value() {
		BevyRuntime::with(|app| {
			app.register_type::<Transform>();
		});

		let (get, set) = BevySignal::signal(Vec3::new(0., 1., 2.));
		let rsx = rsx! {<entity runtime:bevy Transform.translation={get}/>};
		RsxToBevy::spawn(rsx).unwrap();
		set(Vec3::new(3., 4., 5.));

		let mut app = BevyRuntime::with(|app| std::mem::take(app));
		let world = app.world_mut();
		let mut query = world.query::<&Transform>();
		let transform = query.iter(world).next().unwrap();
		expect(transform.translation).to_be(Vec3::new(3., 4., 5.));
	}
}
