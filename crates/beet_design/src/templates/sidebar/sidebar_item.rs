use crate::prelude::*;

#[template]
pub fn SidebarItem(node: SidebarNode, root: bool) -> impl Bundle {
	let SidebarNode {
		display_name,
		path,
		children,
		expanded,
	} = node;
	// let items = tree.
	let class = if root { "root" } else { "" };

	let inner = if children.is_empty()
		&& let Some(path) = path
	{
		rsx! {
			<a class="leaf bm-c-sidebar__link" href=path.to_string()>
				// aria-current={
				// // handled in js but this avoids FOUC
				// entry.isCurrent && 'page'
				// }
				// class:list={[{ large: root }]}
				{display_name}
			</a>
		}
		.any_bundle()
	} else {
		let children = children
			.into_iter()
			.map(|node| SidebarItem { node, root: false })
			.collect::<Vec<_>>();

		let item = if let Some(path) = path {
			rsx! {
				<a class="large bm-c-sidebar__link" href=path.to_string()>
					{display_name}
				</a>
			}
			.any_bundle()
		} else {
			rsx! { <span class="large">{display_name}</span> }.any_bundle()
		};

		rsx! {
			// {group}
			<details class="bm-c-sidebar__sublist" data-always-expand=expanded>
				// open is handled in js
				<summary>{item}</summary>
				<ul class=class>{children}</ul>
			</details>
		}
		.any_bundle()
	};

	rsx! {
		<li class=class>{inner}</li>
		<style src="./sidebar_item.css" />
	}
}
