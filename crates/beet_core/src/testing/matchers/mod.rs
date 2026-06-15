mod close_to;
mod matcher_bool;
mod matcher_close;
mod matcher_control_flow;
mod matcher_eq;
mod matcher_not;
mod matcher_option;
mod matcher_ord;
mod matcher_result;
mod matcher_str;
mod matcher_vec;
pub use close_to::*;
pub use matcher_bool::*;
pub use matcher_close::*;
pub use matcher_control_flow::*;
pub use matcher_eq::*;
pub use matcher_not::*;
pub use matcher_option::*;
pub use matcher_ord::*;
pub use matcher_result::*;
pub use matcher_str::*;
pub use matcher_vec::*;
// Snapshot matchers read/write `.snap` files and use `LazyLock`/`Mutex`, so the
// no_std (embedded) test build drops them; on-device tests use the inline matchers.
#[cfg(feature = "std")]
mod snapshot;
#[cfg(feature = "std")]
pub use snapshot::*;
