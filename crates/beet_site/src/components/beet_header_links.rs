use crate::route_tree;
use beet::prelude::*;


#[template]
pub fn BeetHeaderLinks() -> impl Bundle {
	rsx! {
		<button class="bt-menu-button" id="menu-button">
			Menu
		</button>
		// <Link variant=ButtonVariant::Text href=routes::docs::index()>
		// 	Docs
		// </Link>
		<Link variant=ButtonVariant::Text href=route_tree::blog::index()>
			Blog
		</Link>
		<script hoist:body src="./menu_button.js" />
		<style src="./menu_button.css" />
	}
}
