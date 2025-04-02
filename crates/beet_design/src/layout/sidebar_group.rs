use beet_router::prelude::*;
use beet_rsx::as_beet::*;

#[derive(Clone, Node)]
pub struct SidebarGroup {
	pub tree: StaticRouteTree,
	pub root: bool,
}

fn sidebar_group(SidebarGroup { tree, root }: SidebarGroup) -> RsxNode {
	// let items = tree.
	let class = if root { "root" } else { "" };

	let child_groups = tree.children.into_iter().map(|tree| {
		rsx! {
			<li>
				// {group}
				<details
				class="bm-c-sidebar__sublist"
				// data-always-expand={entry.expanded}
				// open={
				// 	// handled in js but this avoids FOUC
				// 	entry.expanded || Sidebar.flatten(entry.items).some(item => item.isCurrent)}
			>
				<summary>
					<span class="large">{tree.name.clone()}</span>
					// <IconChevronRight class="caret"/>
				</summary>
				{SidebarGroup { tree, root: false }}
				// <Astro.self entries={entry.items}/>
			</details>
			</li>
		}
	});

	let child_links = tree.paths.into_iter().map(|path| {
		let path_str = path.to_string_lossy().to_string();
		rsx! {
			<li>
			<a
			class="bm-c-sidebar__link"
			href={path_str.clone()}
			// aria-current={
			// 	// handled in js but this avoids FOUC
			// 	entry.isCurrent && 'page'
			// }
			// class:list={[{ large: root }]}
			//todo proper name
		>
		{path_str}
			</a>
		</li>
		 }
	});


	rsx! {
		<ul class={class}>
			// {child_groups}
			{child_links}
		</ul>
		<style src="./sidebar_group.css"/>
	}
}



// #[derive(Node)]
// pub struct SidebarGroup {
// 	pub tree: StaticRouteTree,
// }

// fn sidebar_group(SidebarGroup { tree }: SidebarGroup) -> RsxNode {


// }
