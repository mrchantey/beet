use beet_rsx::as_beet::*;

use super::SidebarNode;

#[derive(Clone, Node)]
pub struct SidebarItem {
	pub node: SidebarNode,
	pub root: bool,
}

fn sidebar_item(SidebarItem { node, root }: SidebarItem) -> RsxNode {
	// let items = tree.
	let class = if root { "root" } else { "" };

	let inner = match node {
		SidebarNode::Group {
			display_name,
			path,
			children,
			expanded,
		} => {
			let children = children
				.into_iter()
				.map(|node| SidebarItem { node, root: false });

			let item = if let Some(path) = path {
				rsx! {<a class="large" href={path.to_string_lossy().to_string()}>{display_name}</a>}
			} else {
				rsx! {<span class="large">{display_name}</span>}
			};

			rsx! {
					// {group}
					<details
					class="bm-c-sidebar__sublist"
					data-always-expand={expanded}
					// open={
					// 	// handled in js but this avoids FOUC
					// 	entry.expanded || Sidebar.flatten(entry.items).some(item => item.isCurrent)}
				>
					<summary>
						{item}
						// <IconChevronRight class="caret"/>
					</summary>
					<ul class={class}>
					{children}
					</ul>
					// <Astro.self entries={entry.items}/>
				</details>
			}
		}

		SidebarNode::Route { display_name, path } => {
			rsx! {
				<a
				class="bm-c-sidebar__link"
				href={path.to_string_lossy().to_string()}
				// aria-current={
				// 	// handled in js but this avoids FOUC
				// 	entry.isCurrent && 'page'
				// }
				// class:list={[{ large: root }]}
			>
			{display_name}
				</a>
			 }
		}
	};

	rsx! {
		<li>
		{inner}
		</li>
		<style src="./sidebar_item.css"/>
	}
}
