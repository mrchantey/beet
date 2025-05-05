use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;



pub struct BeetConfigToNativeCodegen;

impl Pipeline<BeetConfig, Result<()>> for BeetConfigToNativeCodegen {
	fn apply(self, config: BeetConfig) -> Result<()> {
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
			.file_group
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(FuncTokensToRsxRoutes::new(value.codegen.clone()))?
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
		let codegen = file_group.apply_codegen().unwrap();
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
