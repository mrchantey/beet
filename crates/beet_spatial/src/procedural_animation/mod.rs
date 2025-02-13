pub mod play_procedural_animation;
pub use self::play_procedural_animation::*;
pub mod procedural_animation_speed;
pub use self::procedural_animation_speed::*;
pub mod serde_curve;
pub use self::serde_curve::*;
pub mod set_curve_on_run;
pub use self::set_curve_on_run::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


pub fn procedural_animation_plugin(app: &mut App) {
	app.add_systems(Update, play_procedural_animation.in_set(TickSet));
}
