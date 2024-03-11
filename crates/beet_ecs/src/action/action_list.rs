use super::action::Action;
use bevy_app::App;
use bevy_ecs::schedule::ScheduleLabel;


pub trait ActionList: 'static {
	fn into_action(self) -> Box<dyn Action>;
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone);
}
// impl<T> ActionList for T where T:Action{
// 		fn into_action(self) -> Box<dyn Action> {
				
// 		}

// 		fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone) {

// 		}
// }


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
	($name:ident, [$($variant:ident),*]) => {
		#[allow(unused_imports)]
		use beet::prelude::*;
		#[allow(unused_imports)]
		use beet::exports::*;
		use beet::exports::bevy_ecs::schedule::ScheduleLabel;
		use beet::exports::bevy_app::App;
		//these should match most action auto impls, see macros/src/action/parse_action.rs
		// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumIter, Display)]
		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumIter, Display, FieldUi)]
		#[hide_ui]
		pub enum $name {
			$($variant($variant),)*
		}

		impl ActionList for $name {
			fn into_action(self) -> Box<dyn Action> {
				match self {
					$(Self::$variant(x) => Box::new(x),)*
				}
			}
			fn add_systems(app:&mut App, schedule: impl ScheduleLabel + Clone){
				for action in Self::iter().map(|item| item) {
					app.add_systems(
						schedule.clone(),
						action.tick_system().in_set(TickSet),
					);
					app.add_systems(
						schedule.clone(),
						action.post_tick_system().in_set(TickSyncSet),
					);
				}
			}
		}

		impl Action for $name {
			fn duplicate(&self) -> Box<dyn Action>{
				match self {
					$(Self::$variant(x) => x.duplicate(),)*
				}
			}

			fn spawn(&self, entity: &mut EntityWorldMut<'_>){
				match self {
					$(Self::$variant(x) => x.spawn(entity),)*
				}
			}
			fn spawn_with_command(&self, entity: &mut EntityCommands){
				match self {
					$(Self::$variant(x) => x.spawn_with_command(entity),)*
				}
			}

			// fn pre_tick_system(&self) -> SystemConfigs;
			fn tick_system(&self) -> SystemConfigs{
				match self {
					$(Self::$variant(x) => x.tick_system(),)*
				}
			}
			fn post_tick_system(&self) -> SystemConfigs{
				match self {
					$(Self::$variant(x) => x.post_tick_system(),)*
				}
			}
			fn meta(&self) -> ActionMeta{
				match self {
					$(Self::$variant(x) => x.meta(),)*
				}
			}
		}

		// impl IntoAction for $name {
		// 	fn into_action(self) -> Box<dyn Action> {
		// 		match self {
		// 			$(Self::$variant(x) => Box::new(x),)*
		// 		}
		// 	}
		// 	fn into_action_ref(&self) -> &dyn Action {
		// 		match self {
		// 			$(Self::$variant(x) => x,)*
		// 		}
		// 	}
		// 	fn into_action_mut(&mut self) -> &mut dyn Action {
		// 		match self {
		// 			$(Self::$variant(x) => x,)*
		// 		}
		// 	}
		// }

		$(
			impl Into<$name> for $variant {
				fn into(self) -> $name {
						$name::$variant(self)
				}
			}
		)*


	};
}

// #[macro_export]
// macro_rules! action_list_internal {
// 	($name:ident, [$($variant:ident),*]) => {
// 		//these should match most action auto impls, see macros/src/action/parse_action.rs
// 		// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumIter, Display)]
// 		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumIter, Display, FieldUi)]
// 		#[hide_ui]
// 		pub enum $name {
// 			$($variant($variant),)*
// 		}

// 		impl IntoAction for $name {
// 			fn into_action(self) -> Box<dyn Action> {
// 				match self {
// 					$(Self::$variant(x) => Box::new(x),)*
// 				}
// 			}
// 			fn into_action_ref(&self) -> &dyn Action {
// 				match self {
// 					$(Self::$variant(x) => x,)*
// 				}
// 			}
// 			fn into_action_mut(&mut self) -> &mut dyn Action {
// 				match self {
// 					$(Self::$variant(x) => x,)*
// 				}
// 			}
// 		}

// 		$(
// 			impl Into<$name> for $variant {
// 				fn into(self) -> $name {
// 						$name::$variant(self)
// 				}
// 			}
// 		)*


// 	};
// }
