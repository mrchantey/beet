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
		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumIter, Display, FieldUi)]
		#[hide_ui]
		pub enum $name {
			$($variant($variant),)*
		}

		impl IntoAction for $name {
			fn into_action(self) -> Box<dyn Action> {
				match self {
					$(Self::$variant(x) => Box::new(x),)*
				}
			}
			fn into_action_ref(&self) -> &dyn Action {
				match self {
					$(Self::$variant(x) => x,)*
				}
			}
			fn into_action_mut(&mut self) -> &mut dyn Action {
				match self {
					$(Self::$variant(x) => x,)*
				}
			}
		}

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
