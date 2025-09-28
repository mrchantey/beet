use beet_core::prelude::*;



fn main() {
	PrettyTracing::default().init();
	trace!("This is Trace");
	info!("This is Info");
	debug!("This is Debug");
	warn!("This is Warn");
	error!("This is Error");
}
