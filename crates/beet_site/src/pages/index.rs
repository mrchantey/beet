use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl IntoHtml {
	let a = 23;
	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHead slot="head"/>
				<BeetHeaderLinks slot="header-nav" />
				<div class="container">
				<h1>Beet</h1>
				// <img style="width:10em" src="/assets/branding/logo.png"/>
				<p><b>"A folk technology framework"</b></p>
				<Card style:cascade class="hero">
				<span style="display: flex; align-items: center; justify-content: center;padding:0;">"🚧 Mind your step! 🚧"</span>
				<p>"Beet is under construction, if this project is of interest please come and say hi in the"<a href="https://discord.gg/DcURUQCXtx">Beetmash Discord Server</a>.</p>
					<footer>
						<Link
							style:cascade
							variant=ButtonVariant::Outlined
							href="https://github.com/mrchantey/beet"
							>Github</Link>
						// <Link
						// 	style:cascade
						// 	variant=ButtonVariant::Primary
						// 	href=routes::docs::index()
						// 	>Get Started</Link>
					</footer>
				</Card>
			Beet is a framework for building technologies that can people can share, access and make their own like other forms of folk culture like music and story. Beet uses the Entity Component System architecture of Bevy, a modular game engine, to provide a common agnostic paradigm for state and behavior across domains like web, games and robotics.
				<h2>Features</h2>
				<h3>"Web UI"</h3>
					<ClientCounter client:load initial=1 />
					<pre node:code lang="rust" src="../content/web-ui.rs"/>
				<h3>"Server Actions"</h3>
				<p>"See your requests in the network tab"</p>
				<ServerCounter client:load initial=1 />
					<pre node:code lang="rust" src="../content/server-actions.rs"/>
				<h3>"Control Flow"</h3>
					<pre node:code lang="rust" src="../content/realtime-agents.rs"/>
				</div>
			</ContentLayout>
		</BeetContext>
		<style>
		.container{
			display: flex;
			flex-direction: column;
			align-items: center;
			padding-top: 3.em;
			gap:1.em;
		}
		.hero{
			width: 30.em;
			max-width: 30.em;
		}
		.hero>footer{
			display: flex;
			flex-direction: row;
			gap: 1.em;
			justify-content: space-between;
			align-items: stretch;
		}
		.hero>footer>a{
			flex: 1;
		}
		.interactivity{
			display: flex;
			flex-direction: row;
			gap: 1.em;
		}
		</style>
	}
}
