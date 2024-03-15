/// Define an action list. This macro accepts a name and a list of actions.
///
/// ```rust
///
/// action_list!(AgentNodes, [
/// 	Run,
/// 	Hide,
/// 	ChooseWhatToDo
/// ]);
/// ```
///
#[macro_export]
macro_rules! action_list {
	($name:ident, [$($variant:tt),*]) => {
		#[allow(unused_imports)]
		use ::beet_ecs::prelude::*;
		#[allow(unused_imports)]
		use ::beet_ecs::exports::*;
		#[derive(Debug, Clone)]// must be debug and clone to be ActionList
		pub struct $name;

		impl ActionSystems for $name {
			fn add_systems(app:&mut App, schedule: impl ScheduleLabel + Clone){
				$($variant::add_systems(app,schedule.clone());)*
			}
		}
		impl ActionTypes for $name {
			fn register(registry:&mut TypeRegistry){
				$($variant::register(registry);)*
			}
		}
	};
}
