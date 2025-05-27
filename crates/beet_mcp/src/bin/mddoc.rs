use rustdoc_md::rustdoc_json_to_markdown;
use rustdoc_md::rustdoc_json_types::Crate;
use sweet::prelude::FsExt;
use sweet::prelude::ReadFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let crate_name = "beet_mcp";
	let json_path = format!("target/doc/doc/{crate_name}.json");
	let file = ReadFile::to_string(&json_path)?;
	let data: Crate = serde_json::from_str(&file)?;

	let markdown = rustdoc_json_to_markdown(data);

	let md_path = format!("target/doc/md/{crate_name}.md");
	FsExt::write(&md_path, &markdown)?;
	println!("Wrote markdown to {md_path}");

	Ok(())
}
