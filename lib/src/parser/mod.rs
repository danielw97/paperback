use std::{collections::HashMap, sync::OnceLock};

use anyhow::Result;

use crate::document::{Document, ParserContext, ParserFlags};

pub mod epub;
pub mod html;
pub mod markdown;
pub mod text;
mod utils;

pub trait Parser: Send + Sync {
	fn name(&self) -> &str;
	fn extensions(&self) -> &[&str];
	fn supported_flags(&self) -> ParserFlags;
	fn parse(&self, context: &ParserContext) -> Result<Document>;
}

#[derive(Clone)]
pub struct ParserInfo {
	pub name: String,
	pub extensions: Vec<String>,
	pub flags: ParserFlags,
}

pub struct ParserRegistry {
	parsers: HashMap<String, Box<dyn Parser>>,
}

impl ParserRegistry {
	fn new() -> Self {
		Self { parsers: HashMap::new() }
	}

	pub fn register<P: Parser + 'static>(&mut self, parser: P) {
		let name = parser.name().to_string();
		self.parsers.insert(name, Box::new(parser));
	}

	pub fn get_parser(&self, name: &str) -> Option<&dyn Parser> {
		self.parsers.get(name).map(|p| &**p)
	}

	pub fn get_parser_for_extension(&self, extension: &str) -> Option<&dyn Parser> {
		let ext = extension.to_lowercase();
		self.parsers.values().find(|p| p.extensions().iter().any(|e| e.to_lowercase() == ext)).map(|p| &**p)
	}

	pub fn all_parsers(&self) -> Vec<ParserInfo> {
		self.parsers
			.values()
			.map(|p| ParserInfo {
				name: p.name().to_string(),
				extensions: p.extensions().iter().map(|s| s.to_string()).collect(),
				flags: p.supported_flags(),
			})
			.collect()
	}

	pub fn global() -> &'static ParserRegistry {
		static REGISTRY: OnceLock<ParserRegistry> = OnceLock::new();
		REGISTRY.get_or_init(|| {
			let mut registry = ParserRegistry::new();
			registry.register(epub::EpubParser);
			registry.register(text::TextParser);
			registry.register(markdown::MarkdownParser);
			registry.register(html::HtmlParser);
			registry
		})
	}
}

pub fn parse_document(context: &ParserContext) -> Result<Document> {
	let path = std::path::Path::new(&context.file_path);
	let extension = path
		.extension()
		.and_then(|e| e.to_str())
		.ok_or_else(|| anyhow::anyhow!("No file extension found for: {}", context.file_path))?;
	let parser = ParserRegistry::global()
		.get_parser_for_extension(extension)
		.ok_or_else(|| anyhow::anyhow!("No parser found for extension: .{}", extension))?;
	let mut doc = parser.parse(context)?;
	doc.compute_stats();
	Ok(doc)
}

pub fn get_all_parsers() -> Vec<ParserInfo> {
	ParserRegistry::global().all_parsers()
}

pub fn get_parser_name_for_extension(extension: &str) -> Option<String> {
	ParserRegistry::global().get_parser_for_extension(extension).map(|p| p.name().to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_registry_has_parsers() {
		let parsers = get_all_parsers();
		assert!(!parsers.is_empty(), "Registry should have parsers");
	}

	#[test]
	fn test_extension_lookup() {
		assert!(get_parser_name_for_extension("txt").is_some());
		assert!(get_parser_name_for_extension("md").is_some());
	}
}
