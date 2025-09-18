use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> impl Bundle {
	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHeaderLinks slot="header-nav" />
				<div class="container">
				<h1>Beet</h1>
				<p><b>"The Bevy Expansion Pack"</b></p>
				<Card style:cascade class="hero">
				<p>"Fullstack Bevy with ECS at every layer of the stack."
				<br/><br/>
				<span style="display: flex; align-items: center; justify-content: center;padding:0;">"🚧 Mind your step! 🚧"</span>
				"Beet is under construction, if this project is of interest please come and say hi in the"<a href="https://discord.com/channels/691052431525675048/1333204907414523964">bevy/beet discord channel</a>.</p>
					<footer>
						<Link
							style:cascade
							variant=ButtonVariant::Outlined
							href="https://github.com/mrchantey/beet"
							>Github</Link>
						<Link
							style:cascade
							variant=ButtonVariant::Primary
							href=routes::docs::index()
							>Get Started</Link>
					</footer>
				</Card>
				// <h2>A Very Bevy Metaframework</h2>
				// <ul>
				// <li>"Very Bevy Licencing"<br/>"Beet inherits Bevy's MIT/Apache licenses"</li>
				// <li>"Very Bevy Architecture"<br/>"Bevy primitives all the way down"</li>
				// </ul>
				<h2>"Very Bevy Web UI"</h2>
					<ClientCounter client:load initial=1 />
					<pre node:code lang="rust" src="../content/web-ui.rs"/>
				<h2>"Very Bevy Server Actions"</h2>
				<p>"See your requests in the network tab"</p>
				<ServerCounter client:load initial=1 />
					<pre node:code lang="rust" src="../content/server-actions.rs"/>
				<h2>"Very Bevy Control Flow"</h2>
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
