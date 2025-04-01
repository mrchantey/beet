use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> RsxNode {
	rsx! {
		<BeetContext>
		<ContentLayout>
			<h1>Beet</h1>
			<a href={paths::docs()}>Get Started</a>
			<Counter client:load initial=2/>
			</ContentLayout>
		</BeetContext>
	}
}
