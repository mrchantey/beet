/// Generates a component `on_add` hook that registers entity observers.
///
/// Pair with `#[component(on_add = hook_name)]` on your struct.
///
/// ## Example
/// ```ignore
/// observer_adder!(on_add_my_action, my_observer);
///
/// #[derive(Component)]
/// #[component(on_add = on_add_my_action)]
/// struct MyAction;
/// ```
#[macro_export]
macro_rules! observer_adder {
	($fn_name:ident, $($observer:expr),+ $(,)?) => {
		#[allow(non_snake_case)]
		fn $fn_name(
			mut world: $crate::prelude::DeferredWorld,
			cx: $crate::prelude::HookContext,
		) {
			world
				.commands()
				.entity(cx.entity)
				$(.observe_any($observer))*;
		}
	};
}
