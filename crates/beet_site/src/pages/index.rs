use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl Bundle {

	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHeaderLinks slot="header-nav" />
				<div class="container">
				<h1>Beet</h1>
				<p>"🌱 A very bevy metaframework 🌱"</p>
				<Link
					class="primary-action"
					href="https://github.com/mrchantey/beet/blob/main/CONTRIBUTING.md"
					style:cascade
					>Contributing</Link>
				<p>"🚧 Mind your step! Beet is under construction, this site is currently for testing and feedback purposes only 🚧"</p>
				// <Link
				// 	class="primary-action"
				// 	href=paths::docs::index()
				// 	style:cascade
				// 	>Get Started</Link>
				<h6>Interactivity Tests</h6>
				<div class="interactivity">
				<Counter client:load initial=1 />
				<Calculator client:load initial=1 />
				</div>
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
		h6{
			padding-top: 2.em;
		}
		.interactivity{
			display: flex;
			flex-direction: row;
			gap: 1.em;
		}


		.primary-action{
			max-width:20.em;
		}

		</style>
	}
}
