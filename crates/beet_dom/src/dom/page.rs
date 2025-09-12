use beet_utils::prelude::*;
use bevy::prelude::*;

pub struct Page {
	// provider: Box<dyn PageProvider>,
}



pub trait PageProvider: 'static + Send + Sync {
	fn visit(&self) -> SendBoxedFuture<Result<()>>;
}
