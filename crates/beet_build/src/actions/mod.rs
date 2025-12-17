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

mod server;
pub use server::*;
mod source_files;
pub use source_files::*;
mod sst;
pub use sst::*;
mod lambda;
pub use lambda::*;
mod wasm;
pub use wasm::*;
mod child_process;
pub use child_process::*;
mod launch_config;
pub use launch_config::*;
mod sync_buckets;
pub use sync_buckets::*;
mod cli_plugin;
pub use cli_plugin::*;
pub use default_cli_router::*;
mod default_cli_router;
