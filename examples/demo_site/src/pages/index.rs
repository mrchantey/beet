use crate::prelude::*;
use beet::prelude::*;





pub fn get() -> impl Bundle {
	let num_requests = AppState::get().num_requests;

	rsx! {
		<Layout>
		<h1>Beet Demo Site</h1>
		<div>
			"Greetings visitor " {num_requests}
		</div>
		<ClientCounter client:load/>
		<ServerCounter client:load/>
		</Layout>
	}
}
