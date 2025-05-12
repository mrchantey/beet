pub mod process;
pub mod terminal;

pub mod prelude {
	pub use crate::exports;
	pub use crate::process::*;
	pub use crate::terminal;
}


pub mod exports {
	pub use notify;
	pub use notify_debouncer_full;
}
