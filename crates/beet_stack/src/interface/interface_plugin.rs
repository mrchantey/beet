use super::*;
// use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct InterfacePlugin;


impl Plugin for InterfacePlugin {
	fn build(&self, app: &mut App) { app.add_observer(single_current_card); }
}
