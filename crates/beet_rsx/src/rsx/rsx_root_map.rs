use crate::prelude::*;




pub struct RsxRootMap(pub HashMap<RsxLocation, RsxRoot>);



impl RsxRootMap {
	pub fn from_roots(roots: Vec<RsxRoot>) -> Self {
		Self(
			roots
				.into_iter()
				.map(|root| (root.location.clone(), root))
				.collect(),
		)
	}
}
