use crate::prelude::*;
use beet_rsx::prelude::*;

#[derive(Default)]
pub struct DefaultFileFuncsToRoutes {
	pub base_route: RoutePath,
	pub transform_path: Option<Box<dyn Fn(RoutePath) -> RoutePath>>,
}


impl
	RsxPipeline<
		Vec<FileFunc<DefaultFileFunc>>,
		Vec<(RouteInfo, DefaultFileFunc)>,
	> for DefaultFileFuncsToRoutes
{
	fn apply(
		self,
		routes: Vec<FileFunc<DefaultFileFunc>>,
	) -> Vec<(RouteInfo, DefaultFileFunc)> {
		routes
			.into_iter()
			.map(|func| {
				let mut info = func.into_route_info();
				info.path = self.base_route.join(&info.path);
				if let Some(transform) = &self.transform_path {
					info.path = transform(info.path);
				}
				(info, func.func)
			})
			.collect()
	}
}


