use crate::prelude::*;
pub fn collect_file_routes(router: &mut crate::DefaultFileRouter) {
    router
        .add_route({
            #[path = "routes/contributing.rs"]
            mod route;
            (RouteInfo::new("/contributing", "get"), route::get)
        });
    router
        .add_route({
            #[path = "routes/index.rs"]
            mod route;
            (RouteInfo::new("/", "get"), route::get)
        });
}
