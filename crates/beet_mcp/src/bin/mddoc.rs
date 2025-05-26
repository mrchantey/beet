use rustdoc_md::rustdoc_json_to_markdown;
use rustdoc_md::rustdoc_json_types::Crate;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let crate_name = "beet_mcp";
	let json_path = format!("target/doc/doc/{crate_name}.json");
	let data: Crate = serde_json::from_reader(fs::File::open(json_path)?)?;

	let markdown = rustdoc_json_to_markdown(data);

	let md_path = format!("target/doc/md/{crate_name}.md");
	fs::write(&md_path, markdown)?;
	println!("Wrote markdown to {md_path}");

	Ok(())
}
