//! The `beet run` command will start the process of running a
//! user defined pipeline. There are two main steps in running pipelines:
//!
//! ## 1. Load Launch Scene
//!
//! The first step is to resolve the launch scene, usually located at `<workspace_root>/launch.ron`.
//! This contains all information regarding the project configuration and pipeline definitions.
//! See [`LaunchState`] for more information.
//!
//! ## 2. Run Specified Pipeline
//!
//! Once the user-defined pipelines are loaded, the specified pipeline is run.
//! See [`PipelineSelector`] for more information.
//!
mod launch_scene;
pub use launch_scene::*;
mod cli_plugin;
pub use cli_plugin::*;
mod cli_args;
pub use cli_args::*;
mod terminal_command;
pub use terminal_command::*;
mod pipeline_selector;
pub use pipeline_selector::*;
