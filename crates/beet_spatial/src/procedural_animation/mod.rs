//! Actions for procedural animations like following curves.
mod play_procedural_animation;
pub use self::play_procedural_animation::*;
mod procedural_animation_speed;
pub use self::procedural_animation_speed::*;
mod serde_curve;
pub use self::serde_curve::*;
mod set_curve_on_run;
pub use self::set_curve_on_run::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Add all systems and types for procedural animation actions:
/// - [`PlayProceduralAnimation`]
pub fn procedural_animation_plugin(app: &mut App) {
	app.add_systems(Update, play_procedural_animation.in_set(TickSet));
}
