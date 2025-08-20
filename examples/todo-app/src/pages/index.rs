use crate::prelude::*;
use beet::prelude::*;







pub fn get() -> impl Bundle {
	let num_requests = AppState::get().num_requests;
	rsx! {
		<Layout>
		<h1>So Much Todo</h1>
		<div>
			"Greetings visitor " {num_requests}
		</div>
		<TodoList client:load/>
		<ClientCounter client:load/>
		<ServerCounter client:load/>
		</Layout>
	}
}
