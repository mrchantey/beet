use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> impl IntoHtml {
	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHead slot="head"/>
				<BeetHeaderLinks slot="header-nav" />
				<div class="container">
				<h1>Beet</h1>
				// <img style="width:10em" src="/assets/branding/logo.png"/>
				<p><b>"An ECS Metaframework for the Fuller Stack."</b></p>
				<Card style:cascade class="hero">
				<span style="display: flex; align-items: center; justify-content: center;padding:0;">"🚧 Mind your step! 🚧"</span>
				<p>"Beet is under construction, if this project is of interest please come and say hi in our"<a href="https://discord.com/channels/691052431525675048/1333204907414523964">bevy discord channel</a>.</p>
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
				<h2>"Web UI"</h2>
					<ClientCounter client:load initial=1 />
					<pre node:code lang="rust" src="../content/web-ui.rs"/>
				<h2>"Server Actions"</h2>
				<p>"See your requests in the network tab"</p>
				<ServerCounter client:load initial=1 />
					<pre node:code lang="rust" src="../content/server-actions.rs"/>
				<h2>"Control Flow"</h2>
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
