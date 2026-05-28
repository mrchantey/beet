use crate::prelude::*;

/// The static home page, linking to the other routes via the generated,
/// compile-time-checked [`routes`] module.
pub fn get() -> impl Bundle {
	rsx_direct!{
		<main>
			<h1>"File-based routes"</h1>
			<ul>
				<li><a href={routes::about()}>"About"</a></li>
				<li><a href={routes::guide()}>"Guide"</a></li>
			</ul>
		</main>
	}
}
