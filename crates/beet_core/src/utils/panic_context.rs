use crate::prelude::*;
use std::cell::Cell;

thread_local! {
	/// Whether [`PanicContext::init`] has been called yet.
	/// Whether we are currently in a panic catch scope
	static IN_SCOPE: Cell<bool> = Cell::new(false);
	/// Captures the panic context
	static CONTEXT: Cell<Option<PanicContext>> = Cell::new(None);
}

static INITIALIZED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Cross-platform method for capturing panic info, including in
/// non-unwind contexts like wasm. See [`Self::catch`]
pub struct PanicContext {
	/// The payload downcast to a string if possible
	payload: Option<String>,
	/// The file and linecol of the location if available
	location: Option<FileSpan>,
}

impl PanicContext {
	/// Cross-platform method for capturing panic info, including in
	/// non-unwind contexts like wasm.
	///
	/// ## Note
	/// This method uses [`panic::set_hook`], calling the prev hook if
	/// a panic occurs outside of this scope. Overriding set_hook will break
	/// this method.
	pub fn catch(func: impl FnOnce() -> Result<(), String>) -> PanicResult {
		// 1. init scope
		if INITIALIZED.get().is_none() {
			Self::init();
		}
		CONTEXT.with(|cx| {
			cx.set(None);
		});
		IN_SCOPE.with(|in_scope| in_scope.set(true));

		// 2. run function
		#[cfg(target_arch = "wasm32")]
		let result = js_runtime::catch_no_abort(func);
		#[cfg(not(target_arch = "wasm32"))]
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func));
		// 3. reset scope
		IN_SCOPE.with(|in_scope| in_scope.set(false));

		match result {
			Ok(Ok(())) => PanicResult::Ok,
			Ok(Err(err)) => PanicResult::Err(err),
			Err(_) => {
				// panicked
				let context = CONTEXT
					.with(|cx| cx.take().expect("panicked but no context"));
				PanicResult::Panic {
					payload: context.payload,
					location: context.location,
				}
			}
		}
	}
	fn init() {
		INITIALIZED.get_or_init(|| true);
		let default_hook = std::panic::take_hook();

		std::panic::set_hook(Box::new(move |info| {
			if !IN_SCOPE.with(|in_scope| in_scope.get()) {
				// not in a catch scope, use default hook
				default_hook(info);
				return;
			}
			CONTEXT.with(|cx| {
				let payload =
					crate::utils::panic_context::downcast_str(info.payload());
				let location = info.location().map(FileSpan::new_from_location);
				cx.set(Some(PanicContext { payload, location }));
			});
		}));
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanicResult {
	Ok,
	Err(String),
	Panic {
		payload: Option<String>,
		location: Option<FileSpan>,
	},
}

/// Attempt to downcast a panic payload into a string
pub fn downcast_str(payload: &dyn std::any::Any) -> Option<String> {
	if let Some(str) = payload.downcast_ref::<&str>() {
		Some(str.to_string())
	} else if let Some(str) = payload.downcast_ref::<String>() {
		Some(str.clone())
	} else {
		None
	}
}
