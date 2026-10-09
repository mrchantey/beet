//! Help middleware that renders route documentation as a material widget.
//!
//! When the `--help` (or `?help`) param is present, [`HelpHandler`] collects the
//! scoped [`RouteTree`] into [`RouteEntry`] rows and renders the [`RouteList`]
//! template. That listing has exactly one home: an unmatched path gets
//! [`ContextualNotFound`]'s small [`NotFoundPage`], which names the miss and
//! links here rather than enumerating the url space to whoever asked. Both go
//! through [`LivePage::respond`], so an ancestor layout (the document chrome)
//! wraps them exactly like any other route, and the one listing serves both the
//! CLI `--help` and the web `?help` view.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
// the widget, not the `beet_net` store table
use beet_ui::prelude::Table;
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
	let is_root = path.is_empty();

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
	// the declarations that act before the build belong to the entry, not to
	// a route, so only the root help lists them
	let prescans = match is_root {
		true => prescan_entries(&caller).await?,
		false => Vec::new(),
	};

	let root = spawn_route_list(&caller, &parts, entries, prescans).await?;
	LivePage::respond(root, &caller, parts).await
}

/// The registered [`Prescan`] set as help rows, empty in a world with none.
async fn prescan_entries(caller: &AsyncEntity) -> Result<Vec<PrescanEntry>> {
	caller
		.world()
		.with(|world: &mut World| {
			world
				.get_resource::<PrescanRegistry>()
				.map(PrescanRegistry::describe)
				.unwrap_or_default()
				.into_iter()
				.map(|(tag, description)| PrescanEntry {
					tag: format!("<{tag}>"),
					description: description.to_string(),
				})
				.collect::<Vec<_>>()
		})
		.await
		.xok()
}

/// Fallback handler for an unmatched path: renders the small [`NotFoundPage`],
/// pointing at the help of the nearest ancestor scene route, with a `NOT_FOUND`
/// status.
///
/// It does NOT render the listing itself. See [`NotFoundPage`] for why.
#[action]
pub(crate) async fn ContextualNotFound(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let path = cx.input.path().clone();

	let notice = cx
		.caller
		.with_state::<AncestorQuery<&RouteTree>, Result<_>>(
			move |entity, query| {
				nearest_ancestor_notice(query.get(entity)?, &path).xok()
			},
		)
		.await??;

	let root = spawn_not_found(&cx.caller, cx.input.parts(), notice).await?;
	let mut response =
		LivePage::respond(root, &cx.caller, cx.input.parts().clone()).await?;
	response.parts.status = StatusCode::NOT_FOUND;
	Ok(response)
}

/// What [`NotFoundPage`] says: the path that missed, and the nearest ancestor
/// scene route whose help to send the caller to, if any.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub struct NotFoundNotice {
	/// The path that was not found.
	pub not_found_path: String,
	/// The nearest ancestor scene-route path whose help covers this miss, if
	/// any. `None` sends the caller to the root help.
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

/// One registered [`Prescan`] type as the root help lists it: the tag and
/// what it does before the build.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub(crate) struct PrescanEntry {
	/// The tag as authored, ie `<Secrets>`.
	pub tag: String,
	/// See [`Prescan::describe`].
	pub description: String,
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
/// routes" heading, followed on the root help by the [`PrescanEntry`] rows under
/// "Before the build".
///
/// One template for both the CLI `--help` and the web `?help`, and the ONE place
/// the url space is enumerated: an unmatched path gets [`NotFoundPage`], which
/// links here instead of repeating it. The document chrome
/// (head/sidebar/footer) is the ancestor layout's job, applied by
/// [`LivePage::respond`], so this widget only owns the route listing. The list is
/// a bare fragment that inherits the page `Background`, not a `.card-filled`
/// surface, so the help reads as the conservative app base — the same near-black
/// page as the regular site — rather than a lighter, tinted card tone.
#[template]
pub fn RouteList(
	entries: Vec<RouteEntry>,
	prescans: Vec<PrescanEntry>,
) -> impl Bundle {
	let items: Vec<_> = entries.into_iter().map(route_entry_item).collect();
	let prescans = (!prescans.is_empty()).then(|| prescan_list(prescans));
	rsx! {
		<>
			<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Available routes"</h2>
			<ul>{items}</ul>
			{prescans}
		</>
	}
}

/// The declarations that act before the entry builds, one row each: a
/// registered type at the entry's top level with string attributes and no
/// `bx:cfg`.
fn prescan_list(prescans: Vec<PrescanEntry>) -> impl Bundle {
	let items: Vec<_> = prescans
		.into_iter()
		.map(|entry| {
			rsx! { <li><strong>{entry.tag}</strong>{format!(": {}", entry.description)}</li> }
		})
		.collect();
	rsx! {
		<>
			<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Before the build"</h2>
			<p>"Declared at the entry's top level with string attributes and no bx:cfg, each acts before the entry builds."</p>
			<ul>{items}</ul>
		</>
	}
}

/// The not-found view: the path that missed, and one link to the route listing
/// that covers it. Nothing else.
///
/// Deliberately small, and deliberately not the listing. This used to render the
/// whole [`RouteList`], which made an unmatched path the dearest route on a site:
/// `beet.org`'s 404 was a 147 KB document of 62 route rows, 13 MB of in-flight
/// memory against 6.2 MB for a real page, and ~100 concurrent probes for one of
/// them is what OOM-killed the box on 2026-09-30. It also handed every scanner a
/// map of the url space for the cost of one wrong guess. The listing keeps one
/// home, `?help` / `--help`, which this links to — scoped to the nearest
/// ancestor scene route when there is one, so the link lands on the help that
/// actually covers the miss.
#[template]
pub fn NotFoundPage(notice: NotFoundNotice) -> impl Bundle {
	let not_found_href = format!("/{}", notice.not_found_path);
	// the help that covers this miss: the ancestor scene route's, else the root's
	let (help_href, help_label) = match &notice.ancestor_path {
		Some(ancestor) => (format!("/{ancestor}?help"), format!("/{ancestor}")),
		None => ("/?help".to_string(), "this site".to_string()),
	};
	rsx! {
		<>
			<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Not found"</h2>
			<p {Classes::new([classes::ERROR_TEXT])}>
				"Route "
				<a href=not_found_href.clone()>{not_found_href}</a>
				" not found."
			</p>
			<p>
				"See the routes "
				<a href=help_href>{format!("{help_label} serves")}</a>
				"."
			</p>
		</>
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
		<Table>
			<tr slot="head">
				<th>"name"</th>
				<th>"kind"</th>
				<th>"required"</th>
				{desc_header}
			</tr>
			{rows}
		</Table>
	}
}

/// Spawn the [`RouteList`] inside a themed page as an ephemeral render root,
/// returning its id.
///
/// The `<RouteList>` is wrapped in a [`PageClasses`] root (`PAGE` plus the
/// resolved color scheme) so a bare render with no host layout — the dev CLI
/// `--help`, where the entry declares no `Layout` — resolves the scheme's
/// `Background` base (the conservative app tone) and its foreground, rather than
/// the black-on-black light `:root` fallback. The list inherits that same neutral
/// `Background` on both the web and the terminal, reading as the regular site's
/// near-black page rather than a lighter card surface. A site that renders the
/// help through its own layout simply nests the same scheme, which resolves
/// identically.
///
/// Built through `spawn_template` so the widget's slots and lifecycle resolve,
/// then marked a self-referential [`PageRoot`] so [`LivePage::respond`] walks
/// it (wrapping it in any ancestor layout) and despawns it after rendering.
async fn spawn_route_list(
	caller: &AsyncEntity,
	parts: &RequestParts,
	entries: Vec<RouteEntry>,
	prescans: Vec<PrescanEntry>,
) -> Result<Entity> {
	let parts = parts.clone();
	caller
		.world()
		.with(move |world: &mut World| {
			spawn_page(
				world,
				&parts,
				rsx! { <RouteList entries=entries prescans=prescans/> },
			)
		})
		.await
}

/// Spawn the [`NotFoundPage`] the same way, so the small view wears the same
/// chrome as the listing it links to.
async fn spawn_not_found(
	caller: &AsyncEntity,
	parts: &RequestParts,
	notice: NotFoundNotice,
) -> Result<Entity> {
	let parts = parts.clone();
	caller
		.world()
		.with(move |world: &mut World| {
			spawn_page(world, &parts, rsx! { <NotFoundPage notice=notice/> })
		})
		.await
}

/// Spawn `body` inside a themed [`PageClasses`] root marked a self-referential
/// [`PageRoot`], the shared tail of the two views above.
fn spawn_page(
	world: &mut World,
	parts: &RequestParts,
	body: impl Bundle,
) -> Result<Entity> {
	let page = PageClasses::resolve(parts, &world.resource::<Theme>().clone());
	let mut entity = world.spawn_template(rsx! { <div {page}>{body}</div> })?;
	let id = entity.id();
	PageRoot::insert(&mut entity, vec![id]);
	id.xok()
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

/// Walk path segments from longest to shortest prefix, naming the first ancestor
/// that matches a scene route, so the not-found link lands on the help that
/// covers the miss rather than always on the root's.
///
/// Builds no [`RouteEntry`] rows: a miss costs a tree lookup, not a flatten of
/// the whole url space.
fn nearest_ancestor_notice(
	tree: &RouteTree,
	segments: &[SmolStr],
) -> NotFoundNotice {
	let ancestor_path = (1..segments.len()).rev().find_map(|length| {
		let prefix = &segments[..length];
		tree.find(prefix)
			.filter(|node| node.is_scene())
			.map(|_| prefix.join("/"))
	});
	NotFoundNotice {
		not_found_path: segments.join("/"),
		ancestor_path,
	}
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
			.spawn((Router::with_defaults(), children![
				Increment::bundle(FieldRef::new("count")),
				Decrement::bundle(FieldRef::new("count")),
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

	/// The root help lists the declarations that act before the build, the
	/// set the router registered; a command's help does not.
	#[beet_core::test]
	async fn root_help_lists_the_prescan_set() {
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), children![
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.flush();
		help_body(&mut world, root, "--help")
			.await
			.xpect_contains("Before the build")
			.xpect_contains("&lt;RepoRoot&gt;")
			.xpect_contains("&lt;RequireCfg&gt;")
			.xpect_contains("&lt;TemplateDir&gt;");
		help_body(&mut world, root, "about --help")
			.await
			.xnot()
			.xpect_contains("Before the build");
	}

	/// REGRESSION (39.1): the `main.bsx` markup shape — a bare [`Router`] with an
	/// explicitly declared [`HelpHandler`], no `with_defaults` — must answer a
	/// root `--help` rather than falling through to a matched action.
	#[beet_core::test]
	async fn explicit_help_handler_on_bare_router() {
		let mut world = router_world();
		let root = world
			.spawn(((Router, HelpHandler::default()), children![
				Increment::bundle(FieldRef::new("count"))
			]))
			.flush();

		help_body(&mut world, root, "--help")
			.await
			.xpect_contains("Available routes")
			.xpect_contains("/increment");
	}

	/// REGRESSION: a bare `Router` — the markup form, eg the beet cli's own
	/// `main.bsx` — answers `--help` too, and a command's own `--help` lists that
	/// command's params rather than running its action. [`HelpHandler`] used to be
	/// spelled out by `Router::with_defaults` alone, so `beet build-wasm --help`
	/// dispatched straight into the action's "--out is required" error.
	#[beet_core::test]
	async fn bare_router_scopes_help_to_a_command() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct BuildParams {
			out: Option<String>,
		}
		/// Stands in for a command that errors without its required param.
		#[action]
		#[derive(Default, Clone, Component, Reflect)]
		#[reflect(Component)]
		async fn RequiresOut(
			_cx: ActionContext<RequestParts>,
		) -> Result<String> {
			bevybail!("--out is required")
		}

		let mut world = router_world();
		let root = world
			.spawn(((Router, HelpHandler::default()), children![
				(
					route::exchange("build", RequiresOut),
					ParamsPartial::new::<BuildParams>()
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.flush();

		// `unwrap_str` panics on a non-ok status, so reaching the body at all
		// proves the help intercepted before the action ran.
		help_body(&mut world, root, "build --help")
			.await
			.xpect_contains("/build")
			.xpect_contains("out")
			// scoped to the command: the sibling route is not listed
			.xnot()
			.xpect_contains("about");
	}

	/// The web `?help` query form routes through the same template as the CLI
	/// `--help`: one [`RouteList`] serves both surfaces.
	#[beet_core::test]
	async fn web_help_query_renders_same_route_list() {
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), children![Increment::bundle(
				FieldRef::new("count")
			)]))
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
			.spawn((Router::with_defaults(), children![(
				render_action::fixed_func_route("counter", || {
					Element::new("p").with_inner_text("counter")
				}),
				children![Increment::bundle(FieldRef::new("count"))],
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
			.spawn((Router::with_defaults(), children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![Increment::bundle(FieldRef::new("count"))],
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
			.spawn((Router::with_defaults(), children![(
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
			.spawn((Router::with_defaults(), children![AddField::bundle(
				FieldRef::new("value")
			)]))
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
			.spawn((Router::with_defaults(), children![
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
				Increment::bundle(FieldRef::new("count")),
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
		fn HelpShell() -> impl Bundle {
			rsx! {
				<html>
					<head><meta charset="utf-8"/></head>
					<body><main><Slot/></main></body>
				</html>
			}
		}

		let mut world = router_world();
		world.register_template::<HelpShell>();
		let root = world
			.spawn((
				Router::with_defaults(),
				Layout::of::<HelpShell>(),
				children![Increment::bundle(FieldRef::new("count"))],
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
	async fn not_found_links_to_the_listing_without_repeating_it() {
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), children![Increment::bundle(
				FieldRef::new("count")
			)]))
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
			.xpect_contains("?help")
			// the url space is NOT enumerated: the route the caller missed
			// should not hand them the one they did not ask for
			.xnot()
			.xpect_contains("Available routes")
			.xnot()
			.xpect_contains("/increment");
	}
}
