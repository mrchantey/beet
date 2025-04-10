use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxNode {
	let counter = if cfg!(debug_assertions) {
		rsx! { <Counter client:load initial=2 /> }
		// this is a hack to get the counter to work in dev mode
		// it should be removed when we have a better way to do this
		// "client:load"
	} else {
		Default::default()
		// "client:only"
	};


	rsx! {
		<BeetContext>
			<ContentLayout>
				<BeetHeaderLinks slot="header-nav" />
				<h1>Beet</h1>
				<p>Its a framework</p>
				<Link class="primary-action" href=paths::docs::index()>Get Started</Link>
		{counter}
			</ContentLayout>
		</BeetContext>
		<style>
		.primary-action{
			max-width:3.em;

		}

		</style>
	}
}
