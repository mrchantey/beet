use crate::prelude::*;
use beet::prelude::*;




pub fn routes() -> impl Bundle {
	(children![
		pages_routes(),
		docs_routes(),
		blog_routes(),
		actions_routes(),
		beet_design::mockups::mockups_routes(),
		(PathFilter::new("docs"), article_layout_middleware()),
		(PathFilter::new("blog"), article_layout_middleware()),
		(PathFilter::new("design"), article_layout_middleware()),
		{
			#[cfg(feature = "deploy")]
			(
				HandlerConditions::fallback(),
				bucket_file_handler(),
				AsyncAction::new(async move |mut world, entity| {
					// TODO infer bucket name from infra config
					// #[cfg(debug_assertions)]
					// let bucket_name = "beet-site-bucket-dev";
					// #[cfg(not(debug_assertions))]
					let bucket_name = "beet-site-bucket-prod";

					let provider = S3Provider::create().await;
					world
						.entity_mut(entity)
						.insert(Bucket::new(provider, bucket_name));
					world
				}),
			)
		}
	],)
}
