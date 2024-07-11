#[macro_export]
macro_rules! build_observer_hooks {
	($type:ty,$app:expr, $($expr:expr),*) => {
			$app.world_mut().register_component_hooks::<$type>()
			.on_add(|mut world, entity, _| {
				ActionObserversBuilder::new::<$type>()
					.add_observers(($($expr,)*))
					.build(world.commands(), entity);
			})
			.on_remove(|mut world, entity, _| {
				ActionObserversBuilder::cleanup::<$type>(&mut world,entity);
			});
	};
}
