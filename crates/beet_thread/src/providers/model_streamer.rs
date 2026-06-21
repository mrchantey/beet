use crate::prelude::*;
use beet_core::prelude::*;

/// The wire protocol (and streamer type) a model is driven through.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum ModelApi {
	/// The OpenResponses streaming protocol ([`O11sStreamer`]).
	#[default]
	OpenResponses,
	/// The OpenAI Chat Completions protocol ([`CompletionsStreamer`]).
	Completions,
}

/// The model provider to select a streamer for.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum Provider {
	#[default]
	Gemini,
	OpenAi,
	Ollama,
}

/// A coarse capability/cost tier, mapped to a concrete model per provider.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum ModelSize {
	Small,
	#[default]
	Medium,
	Large,
}

/// Equip an agent with a model streamer selected ergonomically from a
/// [`Provider`], [`ModelApi`], and [`ModelSize`].
///
/// A `#[template]`, so it equips an actor from markup
/// (`<ModelStreamer provider="OpenAi" size="Large"/>`) or rust
/// (`world.spawn_template(ModelStreamer { provider: Provider::OpenAi, ..default() })`).
/// It resolves the model and auth at build time and inserts the matching
/// streamer ([`O11sStreamer`] or [`CompletionsStreamer`]) onto its entity;
/// construct the streamers directly for fine control. Providers without an
/// OpenResponses endpoint (Gemini) fall back to completions. `instructions`
/// (the per-agent system prompt) is forwarded to the streamer when non-empty,
/// so two agents on one model differ by markup alone.
#[template]
pub fn ModelStreamer(
	#[prop(default)] provider: Provider,
	#[prop(default)] api: ModelApi,
	#[prop(default)] size: ModelSize,
	#[prop(into, default)] instructions: String,
) -> impl Bundle {
	OnSpawn::new(move |entity| -> Result {
		let api = effective_api(provider, api);
		let model = ModelDef {
			provider_slug: provider_slug(provider).into(),
			model_slug: model_slug(provider, size).into(),
			url: model_url(provider, api).into(),
			auth: match auth_env(provider) {
				Some(key) => Some(EnvVar::new(key)?),
				None => None,
			},
		};
		// empty instructions (the default) leave the streamer's system prompt unset
		match api {
			ModelApi::OpenResponses => {
				let mut streamer = O11sStreamer::new(model);
				if !instructions.is_empty() {
					streamer = streamer.with_instructions(instructions);
				}
				entity.insert(streamer);
			}
			ModelApi::Completions => {
				let mut streamer = CompletionsStreamer::new(model);
				if !instructions.is_empty() {
					streamer = streamer.with_instructions(instructions);
				}
				entity.insert(streamer);
			}
		};
		Ok(())
	})
}

/// Gemini exposes only the completions endpoint, so it always drives through
/// [`CompletionsStreamer`] regardless of the requested api.
fn effective_api(provider: Provider, api: ModelApi) -> ModelApi {
	match provider {
		Provider::Gemini => ModelApi::Completions,
		_ => api,
	}
}

fn provider_slug(provider: Provider) -> &'static str {
	match provider {
		Provider::Gemini => GeminiProvider::PROVIDER_SLUG,
		Provider::OpenAi => OpenAiProvider::PROVIDER_SLUG,
		Provider::Ollama => OllamaProvider::PROVIDER_SLUG,
	}
}

fn auth_env(provider: Provider) -> Option<&'static str> {
	match provider {
		Provider::Gemini => Some(GeminiProvider::AUTH_ENV),
		Provider::OpenAi => Some(OpenAiProvider::AUTH_ENV),
		Provider::Ollama => None,
	}
}

fn model_slug(provider: Provider, size: ModelSize) -> &'static str {
	match (provider, size) {
		(Provider::OpenAi, ModelSize::Small) => OpenAiProvider::GPT_5_NANO,
		(Provider::OpenAi, ModelSize::Medium) => OpenAiProvider::GPT_5_MINI,
		(Provider::OpenAi, ModelSize::Large) => OpenAiProvider::GPT_5_2,
		(Provider::Gemini, ModelSize::Small) => {
			GeminiProvider::GEMINI_2_5_FLASH
		}
		(Provider::Gemini, ModelSize::Medium) => {
			GeminiProvider::GEMINI_2_5_FLASH
		}
		(Provider::Gemini, ModelSize::Large) => GeminiProvider::GEMINI_2_5_PRO,
		(Provider::Ollama, ModelSize::Small) => {
			OllamaProvider::FUNCTION_GEMMA_270M_IT
		}
		(Provider::Ollama, ModelSize::Medium) => OllamaProvider::GEMMA_2B,
		(Provider::Ollama, ModelSize::Large) => OllamaProvider::QWEN_3_5_9B,
	}
}

fn model_url(provider: Provider, api: ModelApi) -> &'static str {
	match (provider, api) {
		(Provider::OpenAi, ModelApi::OpenResponses) => {
			OpenAiProvider::RESPONSES_URL
		}
		(Provider::OpenAi, ModelApi::Completions) => {
			OpenAiProvider::COMPLETIONS_URL
		}
		(Provider::Ollama, ModelApi::OpenResponses) => {
			OllamaProvider::RESPONSES_URL
		}
		(Provider::Ollama, ModelApi::Completions) => {
			OllamaProvider::COMPLETIONS_URL
		}
		// Gemini only exposes the completions endpoint
		(Provider::Gemini, _) => GeminiProvider::COMPLETIONS_URL,
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Build the `ModelStreamer` template onto a fresh entity and return it.
	fn equip(
		provider: Provider,
		api: ModelApi,
		size: ModelSize,
	) -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		let entity = app
			.world_mut()
			.spawn_template(ModelStreamer {
				provider,
				api,
				size,
				..default()
			})
			.unwrap()
			.id();
		app.world_mut().flush();
		(app, entity)
	}

	/// A markup spread can set the `instructions` string prop, so two agents on
	/// one model differ by author scene alone (the `multi_agent` litmus). Uses
	/// Ollama to avoid an auth env at build.
	#[beet_core::test]
	fn markup_streamer_carries_instructions() {
		use beet_core::prelude::*;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		let source = r#"
<div {Thread}>
	<ActorDef name="Agent" kind="Agent" {ModelStreamer{provider:Ollama, instructions:"be terse"}}/>
</div>
"#;
		BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.world_mut().flush();

		let streamer = app
			.world_mut()
			.query::<&O11sStreamer>()
			.single(app.world())
			.unwrap();
		streamer.instructions().xpect_eq(Some("be terse"));
	}

	#[beet_core::test]
	fn equips_openresponses_streamer() {
		// Ollama needs no auth env, so this resolves without environment setup
		let (app, entity) =
			equip(Provider::Ollama, ModelApi::OpenResponses, ModelSize::Medium);
		let streamer = app.world().get::<O11sStreamer>(entity).unwrap();
		streamer.model_slug().xpect_eq(OllamaProvider::GEMMA_2B);
		streamer
			.provider_slug()
			.xpect_eq(OllamaProvider::PROVIDER_SLUG);
	}

	#[beet_core::test]
	fn equips_completions_streamer_and_gemini_forces_completions() {
		// Ollama completions
		let (app, ollama) =
			equip(Provider::Ollama, ModelApi::Completions, ModelSize::Small);
		app.world()
			.get::<CompletionsStreamer>(ollama)
			.unwrap()
			.model_slug()
			.xpect_eq(OllamaProvider::FUNCTION_GEMMA_270M_IT);

		// Gemini falls back to completions even when OpenResponses is requested.
		// Requires GEMINI_API_KEY, so only assert when it is present.
		if std::env::var(GeminiProvider::AUTH_ENV).is_ok() {
			let (app, gemini) = equip(
				Provider::Gemini,
				ModelApi::OpenResponses,
				ModelSize::Large,
			);
			app.world()
				.get::<CompletionsStreamer>(gemini)
				.unwrap()
				.model_slug()
				.xpect_eq(GeminiProvider::GEMINI_2_5_PRO);
			app.world().get::<O11sStreamer>(gemini).xpect_none();
		}
	}
}
