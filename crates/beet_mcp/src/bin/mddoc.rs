use rustdoc_md::rustdoc_json_to_markdown;
use rustdoc_md::rustdoc_json_types::Crate;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Load the JSON file
	let json_path = "target/doc/doc/beet_mcp.json";
	let data: Crate = serde_json::from_reader(fs::File::open(json_path)?)?;

	// Convert to Markdown
	let markdown = rustdoc_json_to_markdown(data);

	// Save the Markdown file
	fs::write("api_docs.md", markdown)?;
	println!("Documentation converted successfully!");

	Ok(())
}
