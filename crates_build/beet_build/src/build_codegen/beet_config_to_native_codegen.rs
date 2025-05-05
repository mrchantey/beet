use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;



pub struct BeetConfigToNativeCodegen;

impl Pipeline<BeetConfig, Result<()>> for BeetConfigToNativeCodegen {
	fn apply(self, config: BeetConfig) -> Result<()> {
		config.default_site_config.build_native(config.routes()?)?;
		config
			.file_groups
			.into_iter()
			.map(Self::write_file_group)
			.collect::<Result<Vec<_>>>()?;
		Ok(())
	}
}



impl BeetConfigToNativeCodegen {
	fn apply_for_file_group(value: FileGroupConfig) -> Result<CodegenFile> {
		value
			.to_func_tokens()?
			.xpipe(FuncTokensToRsxRoutes::new(value.codegen))?
			.xmap(|(_, codegen)| codegen)
			.xok()
	}
	fn write_file_group(value: FileGroupConfig) -> Result<()> {
		Self::apply_for_file_group(value)?.build_and_write()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let config = BeetConfig::test_config();
		let file_group = config.file_groups[0].clone();
		let codegen =
			BeetConfigToNativeCodegen::apply_for_file_group(file_group)
				.unwrap();
		let str = codegen
			.build_output()
			.unwrap()
			.xmap(|file| prettyplease::unparse(&file));
		// println!("{}", str);
		expect(str).to_contain(
			"RouteFunc::new(RouteInfo::new(\"/docs\", HttpMethod::Get), file0::get)",
		);
	}
}
