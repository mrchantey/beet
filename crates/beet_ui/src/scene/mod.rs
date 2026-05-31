//! Integration of Bevy's first-party scene system into beet's widget stack.
//!
//! Gated behind the `scene` feature so that `beet_core` and the `rsx_direct!`
//! lowering stay free of the heavy, std-only `bevy_scene` layer. The
//! scene-producing `rsx!` macro lowers markup into the types re-exported here.
mod apply_slots;
mod error_scene;
mod into_scene;
pub mod scene_ext;
mod system_scene;
pub use apply_slots::*;
pub use error_scene::*;
pub use into_scene::*;
pub use system_scene::*;
pub use bevy::scene::CommandsSceneExt;
pub use bevy::scene::EntityCommandsSceneExt;
pub use bevy::scene::EntityScene;
pub use bevy::scene::EntityWorldMutSceneExt;
pub use bevy::scene::RelatedScenes;
pub use bevy::scene::Scene;
pub use bevy::scene::SceneComponent;
pub use bevy::scene::SceneList;
pub use bevy::scene::ScenePlugin;
pub use bevy::scene::on;
pub use bevy::scene::template_value;
// re-exported so `rsx!` output can name the child relationship
pub use bevy::prelude::ChildOf;
