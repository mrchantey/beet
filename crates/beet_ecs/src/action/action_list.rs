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
		//these should match most action auto impls, see macros/src/action/parse_action.rs
		// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumIter, Display)]
		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect,Display, FieldUi)]
		#[hide_ui]
		pub enum $name {
			$($variant($variant),)*
		}

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



		impl Action for $name {
			fn duplicate(&self) -> Box<dyn Action>{
				match self {
					$(Self::$variant(x) => x.duplicate(),)*
				}
			}

			fn insert_from_world(&self, entity: &mut EntityWorldMut<'_>){
				match self {
					$(Self::$variant(x) => x.insert_from_world(entity),)*
				}
			}
			fn insert_from_commands(&self, entity: &mut EntityCommands){
				match self {
					$(Self::$variant(x) => x.insert_from_commands(entity),)*
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

		// $(
		// 	impl Into<$name> for $variant {
		// 		fn into(self) -> $name {
		// 				$name::$variant(self)
		// 		}
		// 	}
		// 	// impl<T> From<T> for $name where T: Into<$variant> {
		// 	// 	fn from(val:T) -> $name {
		// 	// 			$name::$variant(val.into())
		// 	// 	}
		// 	// }
		// )*


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
