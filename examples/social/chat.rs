use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.init_plugin::<SocialPlugin>()
		.add_systems(Startup, setup)
		.add_systems(PostUpdate, (on_create, on_change).chain())
		.run();
}

fn setup(mut commands: Commands) {
	commands
		.spawn((Repeat::new(), children![(
			Thread::default(),
			Sequence::new().allow_no_tool(),
			children![
				(User::system(), children![Post::spawn(
					"you are robot, make beep boop noises"
				)]),
				(
					User::agent(),
					post_tool(OllamaProvider::qwen_3_8b().with_instructions(
						r#"
To assist your understanding of the authorship of a post in multi-user environments.
you are provided input in xml format <user name="foo" id="bar>{content}</user>.
Do not respond with this format.
"#
					))
				),
				(User::human(), stdin_post_tool.into_tool()),
			]
		),]))
		.call::<(), Outcome>((), default());
}

#[tool]
fn stdin_post_tool(
	cx: SystemToolIn,
	mut query: SocialQuery,
) -> Result<Outcome> {
	let heading = paint_ext::cyan_bold(format!("\n\nUser > "));
	print!("{heading}");
	std::io::Write::flush(&mut std::io::stdout())?;
	let mut input = String::new();
	std::io::stdin().read_line(&mut input)?;
	query.spawn_post(cx.caller, PostStatus::Completed, input)?;
	Ok(Pass(()))
}

// cursor to track which part of post deltas have already been printed
#[derive(Default, Deref, DerefMut, Component)]
struct StdoutCursor(u32);

fn on_create(
	mut commands: Commands,
	query: Populated<(Entity, &Post), Added<Post>>,
	thread_query: SocialQuery,
) -> Result {
	for (entity, post) in query.iter() {
		commands.entity(entity).insert(StdoutCursor::default());
		let user = thread_query.user_from_post_entity(entity)?;

		if user.kind() != UserKind::Agent {
			continue;
		}
		let post_kind = post.payload().kind();
		if !post_kind.is_display() {
			continue;
		}

		use PostKind::*;
		let suffix = match post_kind {
			Refusal => "refusal >",
			ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
				"thinking.. "
			}
			Media | Url => "media ",
			_ => ">",
		};

		let heading =
			paint_ext::cyan_bold(format!("\n{} {}\n", user.name(), suffix));
		println!("{heading}");
	}

	Ok(())
}

fn on_change(
	mut query: Populated<(Entity, &Post, &mut StdoutCursor), Changed<Post>>,
	thread_query: SocialQuery,
) -> Result {
	for (entity, post, mut cursor) in query.iter_mut() {
		let user = thread_query.user_from_post_entity(entity)?;
		if user.kind() != UserKind::Agent {
			continue;
		}
		if !post.payload().kind().is_display() {
			continue;
		}
		let payload = post.payload().to_string();

		let new_content = &payload[**cursor as usize..];
		use PostKind::*;
		let colored = match post.payload().kind() {
			Refusal => paint_ext::red(new_content),
			ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
				paint_ext::dimmed(new_content)
			}
			_ => new_content.to_string(),
		};

		print!("{}", colored);
		**cursor = payload.len() as u32;
	}

	Ok(())
}
