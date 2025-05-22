use crate::prelude::*;


#[derive(derive_template)]
pub struct Header {
	#[field(into, default = "/".to_string())]
	home_route: String,
}


fn header(Header { home_route }: Header) -> WebNode {
	let Brand { title, .. } = get_context::<Brand>();

	rsx! {
		<header>
			<a class="app-bar-title button-like" href={home_route}>
				// <Logo/>
				{title}
			</a>
			<slot />
			<nav>
				<slot name="nav"/>
			</nav>
		</header>
		<style>
		header {
			height: var(--bt-header-height);
			padding: 0.em calc(var(--bt-content-padding-width) - 5.em);
			display: flex;
			align-items: center;
			justify-content: space-between;
			background-color: var(--bt-color-surface-container-high);
			border-bottom: 1px solid var(--color-outline-variant);
		}

		.app-bar-title {
			font-size: 2rem;
			font-weight: 900;
			background-image: linear-gradient(90deg, var(--bt-color-primary) 45%, var(--bt-color-secondary) 65%);
			color: transparent !important;
			background-clip: text;
			display: flex;
			align-items: center;
		}

		nav {
			display: flex;
			justify-content: flex-end;
			align-items: center;
			font-size:1.2rem;
		}

	</style>
	}
}
