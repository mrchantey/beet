use crate::prelude::*;


pub trait IntoCollection<M> {
	fn into_collection(self) -> impl Collection;
}

pub trait Collection {
	fn register(self, app: &mut BeetApp);
}


impl<F: FnOnce(&mut BeetApp)> Collection for F {
	fn register(self, app: &mut BeetApp) { self(app) }
}
