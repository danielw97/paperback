use std::{fs, path::Path};

use pulldown_cmark::html::push_html;

use super::utils::build_toc_from_headings;
use crate::{
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags},
	html_to_text::{HtmlSourceMode, HtmlToText},
	parser::Parser,
	utils::encoding::convert_to_utf8,
};

pub struct MarkdownParser;

impl Parser for MarkdownParser {
	fn name(&self) -> &str {
		"Markdown Files"
	}

	fn extensions(&self) -> &[&str] {
		&["md", "markdown", "mdown", "mkdn", "mkd"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_TOC | ParserFlags::SUPPORTS_LISTS
	}

	fn parse(&self, context: &ParserContext) -> Result<Document, String> {
		let bytes = fs::read(&context.file_path)
			.map_err(|e| format!("Failed to open Markdown file '{}': {}", context.file_path, e))?;
		if bytes.is_empty() {
			return Err(format!("Markdown file is empty: {}", context.file_path));
		}
		let markdown_content = convert_to_utf8(&bytes);
		let parser = pulldown_cmark::Parser::new(&markdown_content);
		let mut html_content = String::new();
		push_html(&mut html_content, parser);
		let mut converter = HtmlToText::new();
		if !converter.convert(&html_content, HtmlSourceMode::Markdown) {
			return Err(format!("Failed to convert Markdown to text: {}", context.file_path));
		}
		let title =
			Path::new(&context.file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
		let text = converter.get_text();
		let mut buffer = DocumentBuffer::with_content(text);
		let id_positions = converter.get_id_positions().clone();
		for heading in converter.get_headings() {
			let marker_type = match heading.level {
				1 => MarkerType::Heading1,
				2 => MarkerType::Heading2,
				3 => MarkerType::Heading3,
				4 => MarkerType::Heading4,
				5 => MarkerType::Heading5,
				_ => MarkerType::Heading6,
			};
			buffer.add_marker(
				Marker::new(marker_type, heading.offset).with_text(heading.text.clone()).with_level(heading.level),
			);
		}
		for link in converter.get_links() {
			buffer.add_marker(
				Marker::new(MarkerType::Link, link.offset)
					.with_text(link.text.clone())
					.with_reference(link.reference.clone()),
			);
		}
		for list in converter.get_lists() {
			buffer.add_marker(Marker::new(MarkerType::List, list.offset).with_level(list.item_count));
		}
		for list_item in converter.get_list_items() {
			buffer.add_marker(
				Marker::new(MarkerType::ListItem, list_item.offset)
					.with_text(list_item.text.clone())
					.with_level(list_item.level),
			);
		}
		let toc_items = build_toc_from_headings(converter.get_headings());
		let mut doc = Document::new().with_title(title);
		doc.set_buffer(buffer);
		doc.toc_items = toc_items;
		doc.id_positions = id_positions;
		doc.compute_stats();
		Ok(doc)
	}
}
