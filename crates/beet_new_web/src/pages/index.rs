use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl Bundle {
	rsx! {
		<BaseLayout>
			<div>
				<h1>Welcome to Beet!</h1>
				<p>This is a simple example of a newBeet application.</p>
				<a href={routes::docs()}>Visit an internal route</a>
				<ClientCounter client:load initial=0 />
				<ServerCounter client:load initial=0 />
			</div>
			<style>
				div{
					display: flex;
					flex-direction: column;
					align-items: center;
					gap: 1.em;
				}
			</style>
		</BaseLayout>
	}
}
