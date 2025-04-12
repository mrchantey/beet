use crate::prelude::*;
use beet_rsx::as_beet::*;

#[derive(Clone, Node)]
pub struct SidebarItem {
	pub node: SidebarNode,
	pub root: bool,
}

fn sidebar_item(
	SidebarItem {
		node:
			SidebarNode {
				display_name,
				path,
				children,
				expanded,
			},
		root,
	}: SidebarItem,
) -> RsxNode {
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
	} else {
		let children = children
			.into_iter()
			.map(|node| SidebarItem { node, root: false });

		let item = if let Some(path) = path {
			rsx! {
				<a class="large bm-c-sidebar__link" href=path.to_string()>
					{display_name}
				</a>
			}
		} else {
			rsx! { <span class="large">{display_name}</span> }
		};

		// if children.is_e

		rsx! {
			// {group}
			<details class="bm-c-sidebar__sublist" data-always-expand=expanded>
				// open={
				// // handled in js but this avoids FOUC
				// entry.expanded || Sidebar.flatten(entry.items).some(item => item.isCurrent)}
				// <IconChevronRight class="caret"/>
				<summary>{item}</summary>
				<ul class=class>{children}</ul>
			// <Astro.self entries={entry.items}/>
			</details>
		}
	};

	rsx! {
		<li class=class>{inner}</li>
		<style src="./sidebar_item.css" />
	}
}
