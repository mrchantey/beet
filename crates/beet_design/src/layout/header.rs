use crate::prelude::*;


#[template]
pub fn Header(
	#[field(into, default = "/".to_string())] home_route: String,
	config: Res<PackageConfig>,
) -> impl Bundle {
	rsx! {
		<header class="bt-u-print-hidden">
			<slot name="heading">
			<a class="app-bar-title button-like" href={home_route}>
				// <Logo/>
				{config.title.clone()}
			</a>
			</slot>
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

		a.app-bar-title {
			text-decoration: none;
			cursor: pointer;
			color: var(--bt-color-primary);
			outline: none;
			font-weight: 700;
			font-size: 1.6rem;
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
