#![feature(exit_status_error)]
pub mod cargo;
pub mod process;
pub mod terminal;
pub mod prelude {
	pub use crate::cargo::*;
	pub use crate::exports;
	pub use crate::process::*;
	pub use crate::terminal;
}


pub mod exports {
	pub use notify;
	pub use notify_debouncer_full;
}
