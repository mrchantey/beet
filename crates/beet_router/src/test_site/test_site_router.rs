use crate::prelude::*;
pub fn collect_file_routes(router: &mut crate::DefaultFileRouter) {
    router
        .add_route({
            #[path = "/home/pete/me/beet/crates/beet_router/src/test_site/pages/contributing.rs"]
            mod route;
            (
                RouteInfo::new(
                    "/home/pete/me/beet/crates/beet_router/src/test_site/pages/contributing.rs",
                    "get",
                ),
                route::get,
            )
        });
    router
        .add_route({
            #[path = "/home/pete/me/beet/crates/beet_router/src/test_site/pages/index.rs"]
            mod route;
            (
                RouteInfo::new(
                    "/home/pete/me/beet/crates/beet_router/src/test_site/pages/index.rs",
                    "get",
                ),
                route::get,
            )
        });
}
