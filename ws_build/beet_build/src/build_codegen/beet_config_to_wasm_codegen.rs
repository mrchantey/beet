use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;





pub struct BeetConfigToWasmCodegen;

impl Pipeline<BeetConfig, Result<()>> for BeetConfigToWasmCodegen {
	fn apply(self, config: BeetConfig) -> Result<()> {
		config.default_site_config.build_wasm()
	}
}

// if BuildUtils::is_wasm() {
// } else {
