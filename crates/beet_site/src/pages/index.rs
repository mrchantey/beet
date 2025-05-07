use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> WebNode {
	// let counter = if cfg!(debug_assertions) {
	// 	rsx! {}
	// 	// this is a hack to get the counter to work in dev mode
	// 	// it should be removed when we have a better way to do this
	// 	// "client:load"
	// } else {
	// 	Default::default()
	// 	// "client:only"
	// };


	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHeaderLinks slot="header-nav" />
				<div class="container">
				<h1>Beet</h1>
				<p>The framework that grows with you</p>
				<Link
					class="primary-action"
					href=paths::docs::index()
					style:cascade
					>Get Started</Link>
				<Counter client:load initial=1 />
				<Calculator client:load initial=1 />
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


		.primary-action{
			max-width:20.em;
		}

		</style>
	}
}
