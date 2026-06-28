//! Help middleware that renders route documentation as a material widget.
//!
//! When the `--help` (or `?help`) param is present, [`HelpHandler`] collects the
//! scoped [`RouteTree`] into [`RouteEntry`] rows and renders the [`RouteList`]
//! template. [`ContextualNotFound`] renders the same template for an unmatched
//! path, prefixed with a not-found notice. Both go through
//! [`PageRoot::render`], so an ancestor layout (the document chrome) wraps the
//! list exactly like any other route, and the one template serves both the CLI
//! `--help` and the web `?help` view.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// Middleware that intercepts `--help`/`?help` and renders the scoped
/// [`RouteList`] through the layout.
#[action]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn HelpHandler(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();

	if !request.has_param("help") {
		return next.call(request).await;
	}

	let path = request.path().clone();
	let parts = request.parts().clone();

	// the scoped route entries: the subtree under the requested path, else the
	// whole tree, with the `help` route itself filtered out.
	let entries = caller
		.clone()
		.with_state::<AncestorQuery<&RouteTree>, Result<Vec<RouteEntry>>>(
			move |entity, query| {
				let tree = query.get(entity)?;
				let subtree = tree.find_subtree(&path).unwrap_or(tree);
				route_entries(subtree).xok()
			},
		)
		.await??;

	let root = spawn_route_list(&caller, &parts, None, entries).await?;
	PageRoot::render(root, &caller, parts).await
}

/// Fallback handler that renders the [`RouteList`] scoped to the nearest ancestor
/// scene route of an unmatched path, prefixed with a not-found notice. Returns a
/// `NOT_FOUND` status.
#[action]
pub(crate) async fn ContextualNotFound(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let path = cx.input.path().clone();

	let (notice, entries) = cx
		.caller
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(
			move |entity, query| {
				let tree = query.get(entity)?;
				nearest_ancestor_help(tree, &path).xok()
			},
		)
		.await??;

	let root =
		spawn_route_list(&cx.caller, cx.input.parts(), Some(notice), entries)
			.await?;
	let mut response =
		PageRoot::render(root, &cx.caller, cx.input.parts().clone()).await?;
	response.parts.status = StatusCode::NOT_FOUND;
	Ok(response)
}

/// A not-found notice rendered above the [`RouteList`]: the path that missed and
/// the nearest ancestor scene route whose help is shown, if any.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct NotFoundNotice {
	/// The path that was not found.
	pub not_found_path: String,
	/// The nearest ancestor scene-route path whose help is shown, if any.
	pub ancestor_path: Option<String>,
}

/// A single route row in the [`RouteList`], flattened from an [`ActionNode`] into
/// the render-friendly shape the template consumes.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub(crate) struct RouteEntry {
	/// The route path with a leading slash, eg `/counter/increment`.
	pub href: String,
	/// A kind tag rendered beside the path, eg `scene` or an HTTP method.
	pub tag: Option<String>,
	/// Detail rows (`label`, `value`): description, input/output types.
	pub details: Vec<(String, String)>,
	/// The route's params, rendered as a table beneath the details.
	pub params: Vec<RouteParam>,
}

/// One row of a route's params table: a CLI flag / query param with its concrete
/// type and whether it must be supplied.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub(crate) struct RouteParam {
	/// The param name, eg `out-dir` in `--out-dir=…`.
	pub name: String,
	/// The concrete Rust type the param parses into, eg `alloc::string::String`.
	pub kind: String,
	/// Whether the param must be supplied.
	pub required: bool,
	/// A single-character short flag, if any (eg `r` for `-r`).
	pub short: Option<String>,
	/// The param description, if any.
	pub description: Option<String>,
}

/// The help view: a material list of [`RouteEntry`] rows under an "Available
/// routes" heading, optionally prefixed with a [`NotFoundNotice`].
///
/// One template for both the CLI `--help` and the web `?help`: the document
/// chrome (head/sidebar/footer) is the ancestor layout's job, applied by
/// [`PageRoot::render`], so this widget only owns the route listing. The list is
/// a bare fragment that inherits the page `Background`, not a `.card-filled`
/// surface, so the help reads as the conservative app base — the same near-black
/// page as the regular site — rather than a lighter, tinted card tone.
#[template]
pub fn RouteList(
	notice: Option<NotFoundNotice>,
	entries: Vec<RouteEntry>,
) -> impl Bundle {
	let items: Vec<_> = entries.into_iter().map(route_entry_item).collect();
	rsx! {
		<>
			{notice.map(not_found_notice)}
			<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Available routes"</h2>
			<ul>{items}</ul>
		</>
	}
}

/// The not-found preamble: the missing path linked, and the ancestor route whose
/// help follows, if any.
fn not_found_notice(notice: NotFoundNotice) -> impl Bundle {
	let not_found_href = format!("/{}", notice.not_found_path);
	// when help is scoped to an ancestor scene route, name it; otherwise the
	// notice ends at the plain "not found." after the missing-path link.
	let scoped = notice.ancestor_path.map(|ancestor| {
		let ancestor_href = format!("/{ancestor}");
		rsx! {
			" Showing help for "
			<a href=ancestor_href.clone()>{ancestor_href}</a>
			":"
		}
	});
	rsx! {
		<p {Classes::new([classes::ERROR_TEXT])}>
			"Route "
			<a href=not_found_href.clone()>{not_found_href}</a>
			" not found."
			{scoped}
		</p>
	}
}

/// One route row: the path heading with its kind tag, a nested detail list, and
/// (when the route takes params) a params table.
fn route_entry_item(entry: RouteEntry) -> impl Bundle {
	let RouteEntry {
		href,
		tag,
		details,
		params,
	} = entry;
	// the kind tag (eg `[scene]`/`[GET]`) folds into the heading text so the row
	// stays a single link plus a flat detail list.
	let tag = tag.map(|tag| format!(" [{tag}]")).unwrap_or_default();
	let details: Vec<_> = details
		.into_iter()
		.map(|(label, value)| {
			rsx! { <li><strong>{format!("{label}:")}</strong>{format!(" {value}")}</li> }
		})
		.collect();
	rsx! {
		<li>
			<a href=href.clone()>{href}</a>
			{tag}
			{(!details.is_empty()).then(|| rsx! { <ul>{details}</ul> })}
			{(!params.is_empty()).then(|| params_table(params))}
		</li>
	}
}

/// A route's params as a table: name (with any short flag), the concrete `kind`
/// (Rust type), whether it is required, and a description column when any param
/// carries one.
fn params_table(params: Vec<RouteParam>) -> impl Bundle {
	let with_desc = params.iter().any(|param| param.description.is_some());
	let rows: Vec<_> = params
		.into_iter()
		.map(|param| {
			let RouteParam {
				name,
				kind,
				required,
				short,
				description,
			} = param;
			// fold a short flag into the name cell, eg `release (-r)`
			let name = match short {
				Some(short) => format!("{name} (-{short})"),
				None => name,
			};
			let required = if required { "yes" } else { "no" };
			let desc_cell = with_desc
				.then(|| rsx! { <td>{description.unwrap_or_default()}</td> });
			rsx! {
				<tr>
					<td>{name}</td>
					<td>{kind}</td>
					<td>{required.to_string()}</td>
					{desc_cell}
				</tr>
			}
		})
		.collect();
	let desc_header = with_desc.then(|| rsx! { <th>"description"</th> });
	rsx! {
		<table>
			<thead>
				<tr>
					<th>"name"</th>
					<th>"kind"</th>
					<th>"required"</th>
					{desc_header}
				</tr>
			</thead>
			<tbody>{rows}</tbody>
		</table>
	}
}

/// Spawn the [`RouteList`] inside a themed page as an ephemeral render root,
/// returning its id.
///
/// The `<RouteList>` is wrapped in a [`page_classes`] root (`PAGE` plus the
/// resolved color scheme) so a bare render with no host layout — the dev CLI
/// `--help`, where the entry declares no `BsxLayout` — resolves the scheme's
/// `Background` base (the conservative app tone) and its foreground, rather than
/// the black-on-black light `:root` fallback. The list inherits that same neutral
/// `Background` on both the web and the terminal, reading as the regular site's
/// near-black page rather than a lighter card surface. A site that renders the
/// help through its own layout simply nests the same scheme, which resolves
/// identically.
///
/// Built through `spawn_template` so the widget's slots and lifecycle resolve,
/// then marked a self-referential [`PageRoot`] so [`PageRoot::render`] walks
/// it (wrapping it in any ancestor layout) and despawns it after rendering.
async fn spawn_route_list(
	caller: &AsyncEntity,
	parts: &RequestParts,
	notice: Option<NotFoundNotice>,
	entries: Vec<RouteEntry>,
) -> Result<Entity> {
	let parts = parts.clone();
	caller
		.world()
		.with(move |world: &mut World| -> Result<Entity> {
			// an `Option` prop takes the inner value at the call site (auto-`Some`)
			// or is omitted (defaults to `None`); branch on the notice rather than
			// passing the `Option` through.
			let list = match notice {
				Some(notice) => {
					rsx! { <RouteList notice=notice entries=entries/> }
				}
				None => rsx! { <RouteList entries=entries/> },
			};
			let page = page_classes(&parts, &world.resource::<Theme>().clone());
			let mut entity =
				world.spawn_template(rsx! { <div {page}>{list}</div> })?;
			let id = entity.id();
			PageRoot::insert(&mut entity, vec![id]);
			id.xok()
		})
		.await
}

/// Collect a [`RouteTree`] into [`RouteEntry`] rows, excluding the `help` route.
fn route_entries(tree: &RouteTree) -> Vec<RouteEntry> {
	tree.flatten_nodes()
		.into_iter()
		.filter(|node| {
			node.path.annotated_path().last_segment() != Some("help")
		})
		.map(route_entry)
		.collect()
}

/// Flatten one [`ActionNode`] into a [`RouteEntry`]: path + kind tag, then the
/// detail rows (description, non-trivial input/output types, params).
fn route_entry(node: &ActionNode) -> RouteEntry {
	let path = node.path.annotated_path().to_string();
	let tag = if node.is_scene() {
		Some("scene".to_string())
	} else {
		node.method.as_ref().map(|method| method.to_string())
	};

	let mut details: Vec<(String, String)> = Vec::new();
	if let Some(description) = node.description() {
		details.push(("description".into(), description.to_string()));
	}
	// only show input/output for non-trivial, non-exchange, non-scene routes
	let input_type = node.meta.input().type_name();
	let output_type = node.meta.output().type_name();
	let is_trivial = input_type == "()" && output_type == "()";
	let is_exchange =
		input_type.ends_with("Request") && output_type.ends_with("Response");
	if !is_trivial && !is_exchange && !node.is_scene() {
		details.push(("input".into(), input_type.to_string()));
		details.push(("output".into(), output_type.to_string()));
	}
	let params = node
		.params
		.iter()
		.map(|param| RouteParam {
			name: param.name().to_string(),
			kind: param.type_path().to_string(),
			required: param.is_required(),
			short: param.short().map(|short| short.to_string()),
			description: param.description().map(String::from),
		})
		.collect();

	RouteEntry {
		href: format!("/{path}"),
		tag,
		details,
		params,
	}
}

/// Walk path segments from longest to shortest prefix, returning the not-found
/// notice and the route entries for the first ancestor that matches a scene
/// route (else the whole tree).
fn nearest_ancestor_help(
	tree: &RouteTree,
	segments: &[SmolStr],
) -> (NotFoundNotice, Vec<RouteEntry>) {
	let not_found_path = segments.join("/");

	for length in (1..segments.len()).rev() {
		let prefix = &segments[..length];
		if let Some(node) = tree.find(prefix)
			&& node.is_scene()
		{
			let help_tree = tree.find_subtree(prefix).unwrap_or(tree);
			return (
				NotFoundNotice {
					not_found_path,
					ancestor_path: Some(prefix.join("/")),
				},
				route_entries(help_tree),
			);
		}
	}
	(
		NotFoundNotice {
			not_found_path,
			ancestor_path: None,
		},
		route_entries(tree),
	)
}

/// Format a [`RouteTree`] as a help string, listing both scene routes and
/// actions.
///
/// The help route itself is excluded from the listing. The interactive help
/// surfaces render the material [`RouteList`]; this is the plaintext counterpart
/// for non-rendered CLI output.
pub fn format_route_help(tree: &RouteTree) -> String {
	let mut output = String::new();
	output.push_str("Available routes:\n\n");

	let nodes = tree.flatten_nodes();

	let filtered: Vec<&ActionNode> = nodes
		.into_iter()
		.filter(|node| {
			node.path.annotated_path().last_segment() != Some("help")
		})
		.collect();

	if filtered.is_empty() {
		output.push_str("  (none)\n");
		return output;
	}

	for node in filtered {
		format_action_node_text(&mut output, node);
	}

	output
}

/// Format an [`ActionNode`] as plaintext for CLI output.
fn format_action_node_text(output: &mut String, node: &ActionNode) {
	let path = node.path.annotated_path();

	if node.is_scene() {
		output.push_str(&format!("  /{} [scene]\n", path));
	} else {
		output.push_str(&format!("  /{}", path));
		if let Some(method) = &node.method {
			output.push_str(&format!(" [{}]", method));
		}
		output.push('\n');

		if let Some(description) = node.description() {
			output.push_str(&format!("    {}\n", description));
		}

		let input_type = node.meta.input().type_name();
		let output_type = node.meta.output().type_name();
		// Skip Request->Response and scene action signatures
		let is_exchange = input_type.ends_with("Request")
			&& output_type.ends_with("Response");
		if !is_exchange && !node.is_scene() {
			if input_type != "()" {
				output.push_str(&format!("    input:  {}\n", input_type));
			}
			if output_type != "()" {
				output.push_str(&format!("    output: {}\n", output_type));
			}
		}
	}

	for param in node.params.iter() {
		output.push_str(&format!("    {}\n", param));
	}

	output.push('\n');
}

#[cfg(test)]
mod test {
	use super::*;
	#[allow(unused)]
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Request `path` (CLI form), negotiating HTML, returning the rendered body.
	async fn help_body(world: &mut World, root: Entity, path: &str) -> String {
		world
			.entity_mut(root)
			.exchange(
				Request::from_cli_str(path)
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap_str()
			.await
	}

	#[beet_core::test]
	async fn help_lists_routes() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![
				increment(FieldRef::new("count")),
				decrement(FieldRef::new("count")),
			]))
			.flush();

		help_body(&mut world, root, "--help")
			.await
			.xpect_contains("Available routes")
			.xpect_contains("/increment")
			.xpect_contains("/decrement")
			// the help route itself is excluded
			.xnot()
			.xpect_contains("/help");
	}

	/// The web `?help` query form routes through the same template as the CLI
	/// `--help`: one [`RouteList`] serves both surfaces.
	#[beet_core::test]
	async fn web_help_query_renders_same_route_list() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![increment(FieldRef::new(
				"count"
			))]))
			.flush();

		world
			.entity_mut(root)
			.exchange(
				Request::get("?help")
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.unwrap_str()
			.await
			.xpect_contains("Available routes")
			.xpect_contains("/increment");
	}

	#[beet_core::test]
	async fn help_shows_nested_routes() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![(
				render_action::fixed_func_route("counter", || {
					Element::new("p").with_inner_text("counter")
				}),
				children![increment(FieldRef::new("count"))],
			)]))
			.flush();

		help_body(&mut world, root, "--help")
			.await
			.xpect_contains("/counter/increment");
	}

	#[beet_core::test]
	async fn help_scopes_to_subcommand() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![increment(FieldRef::new("count"))],
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.flush();

		// `counter --help` lists only the counter subtree, not sibling routes
		help_body(&mut world, root, "counter --help")
			.await
			.xpect_contains("increment")
			.xnot()
			.xpect_contains("about");
	}

	#[beet_core::test]
	async fn help_renders_param_table_with_concrete_type() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct BuildParams {
			out_dir: Option<String>,
			release: bool,
		}
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![(
				render_action::fixed_func_route("build", || {
					rsx! { <p>"build"</p> }
				}),
				ParamsPartial::new::<BuildParams>(),
			)]))
			.flush();

		help_body(&mut world, root, "--help")
			.await
			// the kebab-cased param name and the table column headers
			.xpect_contains("out-dir")
			.xpect_contains("kind")
			.xpect_contains("required")
			// the concrete Rust type, not the `single` arity
			.xpect_contains("alloc::string::String")
			.xnot()
			.xpect_contains("kind:single");
	}

	#[beet_core::test]
	async fn help_shows_input_output_types() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![add(FieldRef::new("value"))]))
			.flush();

		// add takes i64 input and returns i64
		help_body(&mut world, root, "--help")
			.await
			.xpect_contains("i64");
	}

	#[beet_core::test]
	async fn help_includes_scenes() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
				increment(FieldRef::new("count")),
			]))
			.flush();

		help_body(&mut world, root, "--help")
			.await
			// scene routes carry a [scene] tag, actions still appear
			.xpect_contains("about")
			.xpect_contains("[scene]")
			.xpect_contains("increment");
	}

	/// The help view renders through the ancestor layout: the document chrome
	/// (here a `<main>` from the layout) wraps the route list.
	#[beet_core::test]
	async fn help_renders_through_layout() {
		#[template]
		fn PageLayout() -> impl Bundle {
			rsx! {
				<html>
					<head><meta charset="utf-8"/></head>
					<body><main><Slot/></main></body>
				</html>
			}
		}

		let mut world = router_world();
		let root = world
			.spawn((
				default_router(),
				BaseLayout::<PageLayout>::default(),
				children![increment(FieldRef::new("count"))],
			))
			.flush();

		help_body(&mut world, root, "--help")
			.await
			// the layout chrome wraps the route list content
			.xpect_contains("<meta charset=\"utf-8\"")
			.xpect_contains("<main>")
			.xpect_contains("Available routes")
			.xpect_contains("/increment");
	}

	#[beet_core::test]
	async fn not_found_shows_route_list() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![increment(FieldRef::new(
				"count"
			))]))
			.flush();

		// not-found responds 404, so take the body directly rather than via the
		// ok-only `unwrap_str`.
		world
			.entity_mut(root)
			.exchange(
				Request::from_cli_str("nonexistent")
					.with_header::<header::Accept>(vec![MediaType::Html]),
			)
			.await
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("Available routes")
			.xpect_contains("/increment");
	}

	#[beet_core::test]
	async fn format_route_help_excludes_help_and_lists_routes() {
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![
				increment(FieldRef::new("count")),
				decrement(FieldRef::new("count")),
			]))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();

		format_route_help(&tree)
			.xpect_contains("Available routes")
			.xpect_contains("increment")
			.xpect_contains("decrement");
	}
}
