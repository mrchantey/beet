use crate::exports::anyhow::Result;
use crate::prelude::*;



#[derive(Default)]
pub struct DefaultRunner {
	#[cfg(feature = "server")]
	pub server_actions: Vec<RouteFunc<RegisterAxumRoute<()>>>,
	pub routes: Vec<RouteFunc<RsxRouteFunc>>,
}

impl DefaultRunner {
	#[tokio::main]
	pub async fn run(self) -> Result<()> {

		if args.is_static {
			self.routes
				.xpipe(RouteFuncsToHtml::new(args.html_dir))
				.await?;
		} else {
			#[cfg(feature = "server")]
			{
				use crate::exports::axum::Router;
				let mut router = Router::new();
				for action in self.server_actions {
					router = (action.func)(router);
				}

				BeetServer {
					html_dir: args.html_dir.into(),
					router,
					..Default::default()
				}
				.serve()
				.await?;
			}
			#[cfg(not(feature = "server"))]
			crate::exports::anyhow::bail!(
				"Server feature is not enabled. Please enable the server feature."
			);
		}

		Ok(())
	}
}
