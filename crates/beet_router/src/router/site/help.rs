//! Help middleware that renders route documentation as a material widget.
//!
//! When the `--help` (or `?help`) param is present, [`HelpHandler`] collects the
//! scoped [`RouteTree`] into [`RouteEntry`] rows, each documenting the params a
//! [`RejectUnknownParams`] url space accepts for it, and renders the
//! [`RouteList`] template. That listing has exactly one home: an unmatched path gets
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
#[reflect(Component, DeclaresParams)]
#[component(on_add = MiddlewareList::<Request, Response>::on_add::<Self>)]
pub async fn HelpHandler(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();

	if !request
		.parse_params::<HelpParams>()
		.is_ok_and(|params| params.help)
	{
		return next.call(request).await;
	}

	let path = request.path().clone();
	let parts = request.parts().clone();
	let is_root = path.is_empty();

	// the scoped listing: the subtree under the requested path, else the whole
	// tree, each route beside the params its dispatch path declares.
	let listing = caller
		.clone()
		.with_state::<(AncestorQuery<&RouteTree>, DeclaredParamsQuery), Result<HelpListing>>(
			move |entity, (trees, declared)| {
				let tree = trees.get(entity)?;
				let subtree = tree.find_subtree(&path).unwrap_or(tree);
				HelpListing::new(subtree, &declared).xok()
			},
		)
		.await??;
	// the declarations that act before the build belong to the entry, not to
	// a route, so only the root help lists them
	let prescans = match is_root {
		true => prescan_entries(&caller).await?,
		false => Vec::new(),
	};

	let root = spawn_route_list(&caller, &parts, listing, prescans).await?;
	LivePage::respond(root, &caller, parts).await
}

/// The params the help reads, its own flag and the page's color scheme.
impl DeclaresParams for HelpHandler {
	fn declared_params() -> ParamsPartial {
		ParamsPartial::new::<(HelpParams, ColorSchemeParams)>()
	}
}

/// The flag [`HelpHandler`] answers.
#[derive(Reflect)]
struct HelpParams {
	/// List the routes beneath this one and the params each takes, rather
	/// than calling it.
	help: bool,
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

/// What a help lists: the routes in scope, each with the params only it takes,
/// and the params every one of them inherits, listed once.
///
/// Built from each route's [`ActionNode::declared_params`], the union a
/// [`RejectUnknownParams`] url space checks a request against, so the help
/// documents exactly the params a strict server accepts.
struct HelpListing {
	/// One row per route, the `help` route itself excluded.
	entries: Vec<RouteEntry>,
	/// The params declared above every listed route, by the router's
	/// middleware, the server or the launch it serves.
	inherited: Vec<ParamsGroup>,
}

impl HelpListing {
	/// The listing of every route in `tree`, the `help` route excluded.
	fn new(tree: &RouteTree, query: &DeclaredParamsQuery) -> Self {
		let routes = tree
			.flatten_nodes()
			.into_iter()
			.filter(|node| {
				node.path.annotated_path().last_segment() != Some("help")
			})
			.map(|node| (node, node.declared_params(query)))
			.collect::<Vec<_>>();
		let first = routes
			.first()
			.map(|(_, declared)| declared.declarations())
			.unwrap_or_default();
		// a declarer above every listed route is listed once, beneath them all
		let shared = first
			.iter()
			.map(|declaration| declaration.declarer)
			.filter(|declarer| {
				*declarer != ParamsDeclarer::Route
					&& routes.iter().all(|(_, declared)| {
						declared.declarations().iter().any(|declaration| {
							declaration.declarer == *declarer
						})
					})
			})
			.collect::<Vec<_>>();
		let mut listed = HashSet::default();
		let inherited = ParamsGroup::collect(
			first
				.iter()
				.filter(|declaration| shared.contains(&declaration.declarer)),
			&mut listed,
		);
		let entries = routes
			.into_iter()
			.map(|(node, declared)| {
				RouteEntry::new(node, &declared, &shared, listed.clone())
			})
			.collect();
		Self { entries, inherited }
	}
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
	/// The route's own params, rendered as a table beneath the details.
	pub params: Vec<RouteParam>,
	/// The params this route inherits that not every listed route does, ie
	/// from the server on a `serve` route.
	pub inherited: Vec<ParamsGroup>,
}

impl RouteEntry {
	/// Flatten one [`ActionNode`] into a row: path + kind tag, the detail rows
	/// (description, non-trivial input/output types), its own params, then
	/// those it inherits from a declarer outside `shared`, skipping any param
	/// `listed` already names.
	fn new(
		node: &ActionNode,
		declared: &DeclaredParams,
		shared: &[ParamsDeclarer],
		mut listed: HashSet<String>,
	) -> Self {
		let path = node.path.annotated_path().to_string();
		let tag = if node.is_scene() {
			Some("scene".to_string())
		} else {
			node.method.as_ref().map(|method| method.to_string())
		};

		let mut details: Vec<(String, String)> = Vec::new();
		if let Some(description) = node.description() {
			details
				.push(("description".into(), description.trim().to_string()));
		}
		// only show input/output for non-trivial, non-exchange, non-scene
		// routes, an exchange sequence's step outcome an exchange too
		let input_type = node.meta.input().type_name();
		let output_type = node.meta.output().type_name();
		let is_trivial = input_type == "()" && output_type == "()";
		let is_exchange = node.meta.input_is::<Request>()
			&& (node.meta.output_is::<Response>()
				|| node.meta.output_is::<Outcome<Request, Response>>());
		if !is_trivial && !is_exchange && !node.is_scene() {
			details.push(("input".into(), input_type.to_string()));
			details.push(("output".into(), output_type.to_string()));
		}
		if declared.forwards() {
			details.push((
				"forwards".into(),
				"any other param, to the program it runs".into(),
			));
		}

		// its own params, once each by name
		let mut own = declared
			.declarations()
			.iter()
			.filter(|declaration| declaration.declarer == ParamsDeclarer::Route)
			.flat_map(|declaration| declaration.params.iter())
			.collect::<Vec<_>>();
		own.sort_by(|left, right| left.name().cmp(right.name()));
		own.dedup_by(|left, right| left.name() == right.name());
		listed.extend(own.iter().map(|meta| meta.name().to_string()));
		let inherited = ParamsGroup::collect(
			declared.declarations().iter().filter(|declaration| {
				declaration.declarer != ParamsDeclarer::Route
					&& !shared.contains(&declaration.declarer)
			}),
			&mut listed,
		);

		Self {
			href: format!("/{path}"),
			tag,
			details,
			params: own.into_iter().map(RouteParam::from).collect(),
			inherited,
		}
	}
}

/// The params one declarer above a route reads, ie a server's bind knobs,
/// listed under the declaring type.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
pub(crate) struct ParamsGroup {
	/// The declaring type, ie `CliServer`.
	pub declarer: String,
	/// Its params, in declaration order.
	pub params: Vec<RouteParam>,
}

impl ParamsGroup {
	/// One group per declaring component, each param once: a param `listed`
	/// already names is skipped, and a group left empty dropped.
	fn collect<'a>(
		declarations: impl IntoIterator<Item = &'a ParamsDeclaration>,
		listed: &mut HashSet<String>,
	) -> Vec<Self> {
		declarations
			.into_iter()
			.filter_map(|declaration| {
				let ParamsDeclarer::Component { name, .. } =
					declaration.declarer
				else {
					return None;
				};
				let params = declaration
					.params
					.iter()
					.filter(|meta| listed.insert(meta.name().to_string()))
					.map(RouteParam::from)
					.collect::<Vec<_>>();
				(!params.is_empty()).then(|| Self {
					declarer: name.to_string(),
					params,
				})
			})
			.collect()
	}
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

impl From<&ParamMeta> for RouteParam {
	fn from(meta: &ParamMeta) -> Self {
		Self {
			name: meta.name().to_string(),
			kind: meta.type_path().to_string(),
			required: meta.is_required(),
			short: meta.short().map(|short| short.to_string()),
			description: meta.description().map(String::from),
		}
	}
}

/// The help view: a material list of [`RouteEntry`] rows under an "Available
/// routes" heading, the [`ParamsGroup`]s every route inherits under "Inherited
/// params", and on the root help the [`PrescanEntry`] rows under "Before the
/// build".
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
	inherited: Vec<ParamsGroup>,
	prescans: Vec<PrescanEntry>,
) -> impl Bundle {
	let items: Vec<_> = entries.into_iter().map(route_entry_item).collect();
	let inherited = (!inherited.is_empty()).then(|| inherited_list(inherited));
	let prescans = (!prescans.is_empty()).then(|| prescan_list(prescans));
	rsx! {
		<>
			<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Available routes"</h2>
			<ul>{items}</ul>
			{inherited}
			{prescans}
		</>
	}
}

/// The params every listed route inherits, grouped by declarer.
fn inherited_list(groups: Vec<ParamsGroup>) -> impl Bundle {
	let groups: Vec<_> = groups.into_iter().map(params_group).collect();
	rsx! {
		<>
			<h2 {Classes::new([classes::TEXT_HEADLINE_SMALL])}>"Inherited params"</h2>
			<p>"Taken by every route above, read by the middleware, server or launch declaring them."</p>
			{groups}
		</>
	}
}

/// One declarer's params: its name, then their table.
fn params_group(group: ParamsGroup) -> impl Bundle {
	rsx! {
		<>
			<p>"from "<strong>{group.declarer}</strong></p>
			{params_table(group.params)}
		</>
	}
}

/// The declarations that act before the entry builds, one row each: a
/// registered type beside the entry's root element or within it, with string
/// attributes and no `bx:cfg`.
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
			<p>"Declared beside the entry's root element or within it, with string attributes and no bx:cfg, each acts before the entry builds."</p>
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

/// One route row: the path heading with its kind tag, a nested detail list,
/// its own params table and one per declarer only it inherits from.
fn route_entry_item(entry: RouteEntry) -> impl Bundle {
	let RouteEntry {
		href,
		tag,
		details,
		params,
		inherited,
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
	let inherited: Vec<_> = inherited.into_iter().map(params_group).collect();
	rsx! {
		<li>
			<a href=href.clone()>{href}</a>
			{tag}
			{(!details.is_empty()).then(|| rsx! { <ul>{details}</ul> })}
			{(!params.is_empty()).then(|| params_table(params))}
			{inherited}
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
	listing: HelpListing,
	prescans: Vec<PrescanEntry>,
) -> Result<Entity> {
	let parts = parts.clone();
	let HelpListing { entries, inherited } = listing;
	caller
		.world()
		.with(move |world: &mut World| {
			spawn_page(
				world,
				&parts,
				rsx! { <RouteList entries=entries inherited=inherited prescans=prescans/> },
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
		help_as(world, root, path, MediaType::Html).await
	}

	/// Request `path` (CLI form), negotiating `media`, returning the body.
	async fn help_as(
		world: &mut World,
		root: Entity,
		path: &str,
		media: MediaType,
	) -> String {
		world
			.entity_mut(root)
			.exchange(
				Request::from_cli_str(path)
					.with_header::<header::Accept>(vec![media]),
			)
			.await
			.unwrap_str()
			.await
	}

	/// The flags the `deploy` command reads.
	#[derive(Reflect)]
	#[allow(dead_code)]
	struct DeployParams {
		/// Deploy as an administrator.
		elevated: bool,
		/// Plan without applying.
		dry_run: bool,
	}

	/// The flag a step of the `audit` sequence reads.
	#[derive(Reflect)]
	#[allow(dead_code)]
	struct AuditParams {
		/// Repair what the audit finds.
		fix: bool,
	}

	/// The flag [`VerboseLogger`] reads.
	#[derive(Reflect)]
	#[allow(dead_code)]
	struct VerboseParams {
		/// Log every request.
		verbose: bool,
	}

	/// Stands in for a command.
	#[action]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Command(_cx: ActionContext<Request>) -> Result<Response> {
		Response::ok().xok()
	}

	/// Stands in for a declarer above one route alone, ie a server on `serve`.
	#[derive(Default, Component, Reflect)]
	#[reflect(Component, DeclaresParams)]
	struct VerboseLogger;

	impl DeclaresParams for VerboseLogger {
		fn declared_params() -> ParamsPartial {
			ParamsPartial::new::<VerboseParams>()
		}
	}

	/// A strict CLI: a [`CliServer`] over a router with help and a `stack`
	/// group whose `deploy` takes [`DeployParams`], whose `audit` step
	/// declares [`AuditParams`] and whose `serve` carries a [`VerboseLogger`].
	/// Returns the router.
	fn spawn_cli(world: &mut World) -> Entity {
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<VerboseLogger>();
		let audit_step =
			Action::<Request, Outcome<Request, Response>>::new_pure(
				|cx: ActionContext<Request>| Outcome::Pass(cx.take()).xok(),
			);
		let root = world
			.spawn(((CliServer::default(), RejectUnknownParams), children![(
				(Router, HelpHandler::default()),
				children![(PathPartial::new("stack"), children![
					(
						route::exchange("deploy", Command),
						ParamsPartial::new::<DeployParams>()
					),
					(PathPartial::new("audit"), ExchangeSequence, children![(
						audit_step,
						ParamsPartial::new::<AuditParams>()
					)]),
					(route::exchange("serve", Command), VerboseLogger),
				])]
			)]))
			.flush();
		world.entity(root).get::<Children>().unwrap()[0]
	}

	/// Every param a strict server accepts for a route, its own, its steps'
	/// and those declared above it, is documented by that route's help.
	#[beet_core::test]
	async fn help_lists_every_param_a_strict_server_accepts() {
		let mut world = router_world();
		let router = spawn_cli(&mut world);
		let nodes = RouteTree::of(&world, router)
			.unwrap()
			.flatten_nodes()
			.into_iter()
			.cloned()
			.collect::<Vec<_>>();
		let routes = world.with_state::<DeclaredParamsQuery, _>(|query| {
			nodes
				.iter()
				.map(|node| {
					let names = node
						.declared_params(&query)
						.params()
						.map(|meta| meta.name().to_string())
						.collect::<Vec<_>>();
					(node.path.annotated_path().to_string(), names)
				})
				.collect::<Vec<_>>()
		});
		// the fixture exercises every kind of declarer
		routes
			.iter()
			.flat_map(|(_, names)| names.iter())
			.collect::<HashSet<_>>()
			.xmap(|names| {
				["elevated", "fix", "verbose", "help", "accept", "entry"]
					.iter()
					.all(|name| names.contains(&name.to_string()))
			})
			.xpect_true();
		for (path, names) in routes {
			let body =
				help_body(&mut world, router, &format!("{path} --help")).await;
			names.iter().fold(body, |body, name| {
				body.xpect_contains(format!("<td>{name}"))
			});
		}
	}

	/// A scoped help lists each route's own params beneath it, those only one
	/// route inherits beside its own, and those every route inherits once.
	#[beet_core::test]
	async fn help_page_groups_params_by_declarer() {
		let mut world = router_world();
		let router = spawn_cli(&mut world);
		help_as(&mut world, router, "stack --help", MediaType::Markdown)
			.await
			.xpect_snapshot();
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
