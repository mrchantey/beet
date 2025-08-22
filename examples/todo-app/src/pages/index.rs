use crate::prelude::*;
use beet::prelude::*;







pub fn get() -> impl Bundle {
	let num_requests = AppState::get().num_requests;
	rsx! {
		<Layout>
		// <div>
		// 	"Greetings visitor " {num_requests}
		// </div>
		<TodoList client:load/>
		// <ClientCounter client:load/>
		// <ServerCounter client:load/>
		</Layout>
	}
}
