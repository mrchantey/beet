use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

pub trait ActionEvent: Event + Debug {
	fn action(&self) -> Entity;
	fn origin(&self) -> Entity;
}

impl<T: RunPayload> ActionEvent for OnRun<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}


impl<T: ResultPayload> ActionEvent for OnResult<T> {
	fn action(&self) -> Entity { self.action }
	fn origin(&self) -> Entity { self.origin }
}
