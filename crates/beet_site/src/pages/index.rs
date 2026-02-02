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
				<p><b>"A personal application framework"</b></p>
				<Card style:cascade class="hero">
				<span style="display: flex; align-items: center; justify-content: center;padding:0;">"🚧 Mind your step! 🚧"</span>
				<p>"Beet is under construction, if this project is of interest please come and say hi in the"<a href="https://discord.gg/DcURUQCXtx">Beetmash Discord Server</a>.</p>
					<footer>
					// <iframe width="975" height="548" src="https://www.youtube.com/embed/JWYcoNOvdWE" title="Unifying the Fuller Stack with Entity Component System" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
						<Link
							style:cascade
							variant=ButtonVariant::Outlined
							href="https://github.com/mrchantey/beet"
							>Github</Link>
						<Link
							style:cascade
							variant=ButtonVariant::Primary
							href=routes::blog::index()
							>Blog</Link>
					</footer>
				</Card>
				<iframe src="https://www.youtube.com/embed/a-Sx0aEhDhc" title="Unifying the Fuller Stack with ECS" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
				<p>"Beet is a framework for building user-modifiable applications, like smalltalk or hypercard. Everything from the CLI to the client application is a"<a href="https://bevy.org">"Bevy App"</a>", and all structure and behavior is written in Entity Component System architecture."
				</p>
				<h2>Smoke Tests</h2>
				<ClientCounter client:load initial=1 />
				<ServerCounter client:load initial=1 />
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
