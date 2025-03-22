//! 🥁🥁🥁 This file has been auto generated by the Beet router.
//! 🥁🥁🥁 Any changes will be overridden if the file is regenerated.
pub mod counter;
pub mod contributing;
pub mod index;
pub const COUNTER: &'static str = "/counter";
pub const CONTRIBUTING: &'static str = "/contributing";
pub const INDEX: &'static str = "/";
use beet::prelude::*;
#[allow(unused_imports)]
use crate as beet_site;
#[cfg(not(target_arch = "wasm32"))]
pub fn collect() -> RouteTree<beet::prelude::StaticRoute> {
    RouteTree {
        mod_path: file!().into(),
        children: vec![],
        routes: Vec::new(),
    }
        .add_route((RouteInfo::new("/counter", "get"), counter::get))
        .add_route((RouteInfo::new("/contributing", "get"), contributing::get))
        .add_route((RouteInfo::new("/", "get"), index::get))
}
#[cfg(target_arch = "wasm32")]
pub fn collect() -> ClientIslandMountFuncs {
    ClientIslandMountFuncs::new(
        vec![
            ("/counter", Box::new(|| { beet::exports::ron::de::from_str:: <
            beet_site::components::counter::Counter > ("(initial:7)") ? .render()
            .pipe(RegisterEffects::new(TreeLocation::new(0u32, 0u32, 0u32))) ?; Ok(())
            })), ("/contributing", Box::new(|| { Ok(()) })), ("/", Box::new(|| {
            beet::exports::ron::de::from_str:: < beet_site::components::counter::Counter
            > ("(initial:2)") ? .render()
            .pipe(RegisterEffects::new(TreeLocation::new(94u32, 0u32, 14u32))) ?; Ok(())
            }))
        ],
    )
}
