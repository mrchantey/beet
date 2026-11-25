//! `<RouteTool/>`: a route of the url space surrounding an agent, as one of
//! its tools.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// A route of the url space surrounding the agent, ie `eval/grade`, as one of
/// the agent's tools: a route in the agent's own url space at the same path, so
/// the tool's name is the verb a person runs.
///
/// ```bsx
/// <CreateActor name="Coach" kind="Agent" {ModelStreamer{provider:OpenAi}}>
///     <Router>
///         <RouteTool path="eval/rows"/>
///         <RouteTool path="eval/put" params="table=claims"/>
///     </Router>
/// </CreateActor>
/// ```
///
/// Its definition is the target route's ([`RouteToolCall::definition`]): its
/// doc, and its declared params as the arguments. A dynamic segment after
/// `path`, ie the `*rubric` of `eval/build/*rubric`, is a string argument
/// filled into the path. A call is forwarded to the target and answers what it
/// answers, a refusal included: to a route reading its params off the request,
/// ie an `#[action]` taking `#[param]`s, its arguments are sent as those
/// params, so they are validated as flags are; to a route taking a typed
/// input, as the JSON body that input reads.
///
/// `params` pins params the agent cannot choose, as a query string: each is
/// left out of the schema, sent with every call, and a call supplying one is
/// refused. `accept` is the media type asked of the target, `text/markdown`,
/// an agent's reading form, by default.
#[template]
pub fn RouteTool(
	/// The target route's path, its static segments, and this tool's path in
	/// the agent's url space.
	#[prop(required)]
	path: String,
	/// The params sent with every call, as a query string, ie
	/// `package=acme&table=claims`.
	#[prop(default)]
	params: String,
	/// The media type asked of the target.
	#[prop(default = String::from("text/markdown"))]
	accept: String,
) -> impl Bundle {
	(PathPartial::new(&path), RouteToolCall {
		path: path.into(),
		params: params.into(),
		accept: accept.into(),
	})
}

/// The action of a [`RouteTool`]: each call forwarded to the route at `path`
/// in the url space surrounding the agent ([`ThreadQuery::agent_url_space`]).
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn RouteToolCall(
	/// The target route's static path.
	#[field]
	path: SmolStr,
	/// The pinned params, as a query string.
	#[field]
	params: SmolStr,
	/// The media type asked of the target.
	#[field]
	accept: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let this = RouteToolCall {
		path,
		params,
		accept,
	};
	let world = cx.world();
	let (url_space, captures, params) = cx
		.caller
		.with_state::<(ThreadQuery, Query<&RouteTree>), _>({
			let this = this.clone();
			move |entity, (query, trees)| -> Result<_> {
				let url_space = query.agent_url_space(entity)?;
				let node = this.target(&query, &trees, url_space)?;
				let captures = RouteToolCall::captures(&node.path, &this.path);
				let params = node.reads_request().then(|| node.params.clone());
				(url_space, captures, params).xok()
			}
		})
		.await??;
	let request = this.forward(cx.input, &captures, params.as_ref()).await?;
	world.entity(url_space).exchange(request).await.xok()
}

/// A dynamic segment of a target route after a [`RouteTool`]'s path: a string
/// argument of the tool, filled into the path.
struct Capture {
	name: SmolStr,
	optional: bool,
	greedy: bool,
}

impl RouteToolCall {
	/// The tool definition of this route: the target route's description, and
	/// as its arguments the target's own params less those pinned, beside a
	/// string per dynamic segment after [`path`](Self::path).
	///
	/// ## Errors
	///
	/// Errors when the url space at `url_space` has no such route, or the
	/// route takes a typed input while this pins params or fills captures.
	pub fn definition(
		&self,
		query: &ThreadQuery,
		trees: &Query<&RouteTree>,
		url_space: Entity,
	) -> Result<FunctionToolDefinition> {
		let node = self.target(query, trees, url_space)?;
		let description = node.description().unwrap_or_default();
		let captures = Self::captures(&node.path, &self.path);
		let pinned = self.pinned();
		let schema = match node.reads_request() {
			true => JsonSchema::try_from_schema(&ValueSchema::Struct(
				StructSchema {
					fields: node
						.params
						.own()
						.filter(|param| !pinned.contains_key(param.name()))
						.map(ParamMeta::field_schema)
						.chain(captures.iter().map(Capture::field_schema))
						.collect(),
					..default()
				},
			))?,
			false if pinned.is_empty() && captures.is_empty() => {
				node.meta.input_json_schema()?.ok_or_else(|| {
					bevyhow!(
						"the route `{}` takes no input a tool can describe",
						self.path
					)
				})?
			}
			false => bevybail!(
				"`<RouteTool path=\"{}\"/>` pins params or fills a path segment, \
				 which only a route reading its params off the request takes",
				self.path
			),
		};
		FunctionToolDefinition::new(self.path.as_str(), description, schema)
	}

	/// The params pinned for every call.
	fn pinned(&self) -> MultiMap<SmolStr, SmolStr> {
		Url::parse_query_string(&self.params)
	}

	/// The route of the url space at `url_space` this tool forwards to: the
	/// one whose static segments are [`path`](Self::path), every segment after
	/// them dynamic, a route at exactly `path` preferred.
	fn target<'a>(
		&self,
		query: &ThreadQuery,
		trees: &'a Query<&RouteTree>,
		url_space: Entity,
	) -> Result<&'a ActionNode> {
		let tree = query
			.children
			.iter_descendants_inclusive(url_space)
			.find_map(|entity| trees.get(entity).ok())
			.ok_or_else(|| {
				bevyhow!(
					"no RouteTree at or under the agent's url space {url_space}"
				)
			})?;
		let segments = self
			.path
			.split('/')
			.filter(|segment| !segment.is_empty())
			.collect::<Vec<_>>();
		let mut candidates = tree
			.iter_dfs()
			.into_iter()
			.filter_map(RouteTree::node)
			.filter(|node| {
				let (head, tail) =
					node.path.split_at(segments.len().min(node.path.len()));
				head.len() == segments.len()
					&& head.iter().zip(&segments).all(|(seg, name)| {
						seg.is_static() && seg.name() == *name
					}) && tail.iter().all(|seg| !seg.is_static())
			})
			.collect::<Vec<_>>();
		candidates.sort_by_key(|node| node.path.len());
		candidates.into_iter().next().ok_or_else(|| {
			bevyhow!(
				"`<RouteTool path=\"{}\"/>` names no route of the url space surrounding the agent",
				self.path
			)
		})
	}

	/// Each dynamic segment of `pattern` after the `path` prefix.
	fn captures(pattern: &PathPattern, path: &str) -> Vec<Capture> {
		let skip = path
			.split('/')
			.filter(|segment| !segment.is_empty())
			.count();
		pattern
			.iter()
			.skip(skip)
			.map(|segment| Capture {
				name: segment.name().into(),
				optional: segment.modifier().is_optional(),
				greedy: segment.is_greedy(),
			})
			.collect()
	}

	/// The request forwarded for one `call`: its arguments less the captures,
	/// which fill the path, as the `params` of a target route reading its
	/// params off the request ([`Self::insert_params`]), else as a JSON body,
	/// with the pinned params beside them.
	///
	/// ## Errors
	///
	/// Errors when the call supplies a pinned param, leaves out a required
	/// capture, or sends a body that is not a JSON object.
	async fn forward(
		&self,
		call: Request,
		captures: &[Capture],
		params: Option<&ParamsPattern>,
	) -> Result<Request> {
		let mut fields = match call.body.into_bytes().await? {
			bytes if bytes.iter().all(u8::is_ascii_whitespace) => {
				Map::default()
			}
			bytes => match MediaType::Json.deserialize::<Value>(&bytes)? {
				Value::Map(fields) => fields,
				other => bevybail!(
					"the arguments of `{}` are no JSON object: {other}",
					self.path
				),
			},
		};
		let pinned = self.pinned();
		if let Some(key) = fields.keys().find(|key| {
			pinned.contains_key(key.as_str())
				|| pinned.contains_key(key.replace('_', "-").as_str())
		}) {
			bevybail!(
				"`{key}` is fixed by the tool `{}` and cannot be chosen",
				self.path
			);
		}
		let mut path = self.path.to_string();
		for capture in captures {
			match fields.shift_remove(capture.name.as_str()) {
				Some(Value::Str(value)) => {
					path.push('/');
					path.push_str(value.trim_matches('/'));
				}
				Some(Value::Null) | None if capture.optional => {}
				Some(other) => bevybail!(
					"`{}` of `{}` is a path, a string, not {other}",
					capture.name,
					self.path
				),
				None => bevybail!(
					"`{}` needs the argument `{}`",
					self.path,
					capture.name
				),
			}
		}
		let mut request = Request::get(path.as_str())
			.with_accept(MediaType::from(self.accept.as_str()));
		match params {
			Some(pattern) => Self::insert_params(&mut request, fields, pattern),
			None => {
				request = request
					.with_body(Value::Map(fields).to_json_string())
					.with_content_type(MediaType::Json);
			}
		}
		for (key, values) in pinned.iter_all() {
			for value in values {
				request
					.request_parts_mut()
					.insert_param(key.clone(), value.clone());
			}
		}
		request.xok()
	}

	/// Each argument of a call as a param of `request`, as a flag spells it:
	/// a string its text, a number or bool its JSON spelling, a null absent,
	/// a list one value per item where `pattern` repeats the param, and any
	/// other list or object its JSON text, which a structured param reads
	/// back as the structure it spells.
	fn insert_params(
		request: &mut Request,
		fields: Map,
		pattern: &ParamsPattern,
	) {
		let text = |value: &Value| -> SmolStr {
			match value {
				Value::Str(text) => text.clone(),
				other => other.to_json_string().into(),
			}
		};
		let parts = request.request_parts_mut();
		for (key, value) in fields.0 {
			let values = match value {
				Value::Null => continue,
				Value::List(items) if pattern.repeats(&key) => {
					items.iter().map(text).collect()
				}
				value => vec![text(&value)],
			};
			for value in values {
				parts.insert_param(key.clone(), value);
			}
		}
	}
}

impl Capture {
	/// This capture as a string field of the tool's arguments.
	fn field_schema(&self) -> NamedFieldSchema {
		let what = match self.greedy {
			true => "a path, its segments joined by `/`",
			false => "one path segment",
		};
		let string = ValueSchema::String(default());
		let mut field = match self.optional {
			true => NamedFieldSchema::new(
				self.name.clone(),
				ValueSchema::Optional(Box::new(string)),
			)
			.optional(),
			false => NamedFieldSchema::new(self.name.clone(), string),
		};
		field.description =
			Some(format!("The `{}`, {what}.", self.name).into());
		field
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// Stands in for a verb taking params.
	#[action(route = "eval/grade")]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Grade(
		/// The eval to grade.
		#[param]
		eval: String,
		/// The level reached.
		#[param]
		level: u8,
		/// Grade again though graded.
		#[param]
		force: bool,
		/// The tags to grade under.
		#[param]
		tags: Vec<String>,
		/// How hard to grade.
		#[param]
		strictness: Option<Strictness>,
		/// Where the eval came from.
		#[param]
		source: Option<Url>,
	) -> Result<MediaBytes> {
		let _ = (force, strictness, source);
		MediaBytes::new_text(format!("graded {eval} at {level} {tags:?}")).xok()
	}

	/// How hard [`Grade`] grades.
	#[derive(Debug, Clone, Reflect)]
	enum Strictness {
		Lenient,
		Strict,
	}

	/// Stands in for a verb writing a row of a table.
	#[action(route = "eval/put")]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Put(
		/// The table.
		#[param]
		table: String,
		/// The row.
		#[param]
		row: Value,
	) -> Result<MediaBytes> {
		MediaBytes::new_text(format!(
			"put {} into {table}",
			row.to_json_string()
		))
		.xok()
	}

	/// Stands in for a verb whose target is a path capture.
	#[action(route = "eval/build/*rubric")]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Build(cx: ActionContext<Request>) -> Result<MediaBytes> {
		MediaBytes::new_text(format!("built {}", cx.input.path_string())).xok()
	}

	/// Stands in for a verb the agent is given no tool for.
	#[action(route = "eval/secret")]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Secret() -> Result<MediaBytes> {
		MediaBytes::new_text("leaked").xok()
	}

	/// A router of the four verbs, a thread beneath it whose agent plays
	/// `script` holding a [`RouteTool`] for all but `eval/secret`, returning the
	/// app and the agent.
	fn setup(script: &str) -> (App, Entity, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let router = app
			.world_mut()
			.spawn((Router, children![
				Grade::default(),
				Put::default(),
				Build::default(),
				Secret::default(),
			]))
			.flush();
		let source = format!(
			r#"
<Thread {{Sequence}}>
	<CreateActor name="User" kind="User"><CreatePost text="go"/></CreateActor>
	<CreateActor name="Agent" kind="Agent" {{MockPostStreamer{{script:[{script}]}}}}>
		<Router>
			<RouteTool path="eval/grade"/>
			<RouteTool path="eval/put" params="table=claims"/>
			<RouteTool path="eval/build"/>
		</Router>
	</CreateActor>
</Thread>
"#
		);
		let thread = BsxTemplate::parse_entry(app.world(), &source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.world_mut().entity_mut(router).add_child(thread);
		app.update();
		ThreadRecords::reduce_now(app.world_mut());
		let agent = app
			.world_mut()
			.query_filtered::<Entity, With<MockPostStreamer>>()
			.single(app.world())
			.unwrap();
		(app, thread, agent)
	}

	/// Runs `turns` of the agent, returning each call's output.
	async fn outputs(
		app: &mut App,
		thread: Entity,
		agent: Entity,
		turns: usize,
	) -> Vec<String> {
		for _ in 0..turns {
			app.world_mut()
				.entity_mut(agent)
				.call::<(), Outcome>(())
				.await
				.unwrap();
		}
		app.world()
			.get::<ThreadRecords>(thread)
			.unwrap()
			.posts()
			.iter()
			.filter_map(|post| match post.as_agent_post()? {
				AgentPost::FunctionCallOutput(output) => {
					Some(output.output().to_string())
				}
				_ => None,
			})
			.collect()
	}

	/// The agent's tool at `path`.
	fn tool(
		app: &mut App,
		agent: Entity,
		path: &str,
	) -> FunctionToolDefinition {
		app.world_mut().with_state::<ThreadQuery, _>(|query| {
			query
				.tools(agent)
				.unwrap()
				.into_iter()
				.find_map(|(_, tool)| match tool {
					ToolDefinition::Function(func) if func.path() == path => {
						Some(func)
					}
					_ => None,
				})
				.unwrap()
		})
	}

	/// A verb is a tool at its own path, its params the arguments, and a call
	/// answers what the verb answers.
	#[beet_core::test]
	async fn calls_a_route_with_params() {
		let (mut app, thread, agent) = setup(
			r#"Call{tool:"eval/grade", arguments:{eval:"brand.voice", level:2, tags:["a", "b"]}}"#,
		);
		let grade = tool(&mut app, agent, "eval/grade");
		grade.description().xpect_contains("Stands in for a verb");
		grade
			.params_schema()
			.get("required")
			.unwrap()
			.to_json_string()
			.xpect_eq(r#"["eval","level"]"#);
		outputs(&mut app, thread, agent, 1).await.xpect_eq(vec![
			r#""graded brand.voice at 2 [\"a\", \"b\"]""#.to_string(),
		]);
	}

	/// The target route's params are the tool's schema, each typed as a
	/// structured caller writes it, with its doc line and how its type is
	/// written where that is particular.
	#[beet_core::test]
	fn route_params_are_the_schema() {
		let (mut app, _, agent) = setup("");
		tool(&mut app, agent, "eval/grade")
			.params_schema()
			.to_json_string()
			.xpect_snapshot();
	}

	/// A surrounding route the agent holds no [`RouteTool`] for is its router's
	/// not-found.
	#[beet_core::test]
	async fn refuses_a_route_with_no_tool() {
		let (mut app, thread, agent) =
			setup(r#"Call{tool:"eval/secret", arguments:{}}"#);
		outputs(&mut app, thread, agent, 1).await[0]
			.as_str()
			.xpect_contains("404")
			.xnot()
			.xpect_contains("leaked");
	}

	/// A path capture of the target route is a required string argument,
	/// filled into the path.
	#[beet_core::test]
	async fn fills_a_capture_from_an_argument() {
		let (mut app, thread, agent) = setup(
			r#"Call{tool:"eval/build", arguments:{rubric:"acme/owner"}}"#,
		);
		tool(&mut app, agent, "eval/build")
			.params_schema()
			.get("required")
			.unwrap()
			.to_json_string()
			.xpect_eq(r#"["rubric"]"#);
		outputs(&mut app, thread, agent, 1)
			.await
			.xpect_eq(vec![r#""built /eval/build/acme/owner""#.to_string()]);
	}

	/// A pinned param is left out of the schema and sent with every call, and
	/// a call supplying it is refused.
	#[beet_core::test]
	async fn pins_a_param() {
		let (mut app, thread, agent) = setup(
			r#"Call{tool:"eval/put", arguments:{row:{id:"a"}}},
			Call{tool:"eval/put", arguments:{table:"evals", row:{id:"a"}}}"#,
		);
		let schema = tool(&mut app, agent, "eval/put")
			.params_schema()
			.to_json_string();
		schema
			.as_str()
			.xpect_contains("row")
			.xnot()
			.xpect_contains("table");
		let outputs = outputs(&mut app, thread, agent, 2).await;
		outputs[0]
			.as_str()
			.xpect_eq(r#""put {\"id\":\"a\"} into claims""#);
		outputs[1]
			.as_str()
			.xpect_contains("`table` is fixed by the tool `eval/put`");
	}
}
