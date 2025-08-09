use beet_core::prelude::PrettyTracing;
use tracing::*;



fn main() {
	PrettyTracing::default().init();
	trace!("This is Trace");
	info!("This is Info");
	debug!("This is Debug");
	warn!("This is Warn");
	error!("This is Error");
	beet_utils::log_debug("hello from beet_utils");
}
