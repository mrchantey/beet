//! App plugins for `beet_ml`, eg sharing Bevy's GPU with Burn.
mod shared_burn_wgpu;
pub use shared_burn_wgpu::*;
