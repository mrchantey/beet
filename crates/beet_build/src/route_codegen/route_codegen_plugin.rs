use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default)]
pub struct RouteCodegenPlugin;

impl Plugin for RouteCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<RouteCodegenConfig>()
			.add_systems(
				Update,
				(
					(
						spawn_route_files,
						(parse_route_file_rs, parse_route_file_md),
						modify_file_route_tokens,
					)
						.chain()
						.in_set(BeforeParseTokens),
					(
						parse_route_tree,
						(
							(
								reexport_file_groups,
								add_client_codegen_to_actions_export,
							),
							collect_file_group,
						)
							.chain(),
						collect_client_action_group,
						(collect_combinator_route, tokenize_combinator_route)
							.chain(),
					)
						.in_set(AfterParseTokens),
					#[cfg(not(test))]
					compile_router.after(ExportArtifactsSet),
				),
			);
	}
}
