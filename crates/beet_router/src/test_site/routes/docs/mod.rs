//! 🥁🥁🥁 This file has been auto generated by the Beet router.
//! 🥁🥁🥁 Any changes will be overridden if the file is regenerated.
pub mod index;
use crate::prelude::*;
pub fn collect_file_routes(router: &mut crate::DefaultFileRouter) {
	router.add_route((RouteInfo::new("/docs/", "get"), index::get));
}
