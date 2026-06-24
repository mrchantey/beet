//! Scene-spawn deferral: gates a template's `LoadTemplate` on a built
//! `WorldAssetRoot`'s `WorldInstanceReady`, so spawned scene children are
//! present before any load verb runs.
mod scene_ready;
pub use self::scene_ready::*;
