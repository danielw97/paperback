use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use anyhow::{Context, Result};
use roxmltree::{Document as XmlDocument, Node, NodeType};
use zip::ZipArchive;

use crate::{
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags},
	parser::{Parser, utils::build_toc_from_buffer},
	utils::zip::read_zip_entry_by_name,
};

pub struct OdtParser;

impl Parser for OdtParser {
	fn name(&self) -> &str {
		"OpenDocument Text Files"
	}

	fn extensions(&self) -> &[&str] {
		&["odt"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_TOC
	}

	fn parse(&self, context: &ParserContext) -> Result<Document> {
		let file = File::open(&context.file_path)
			.with_context(|| format!("Failed to open ODT file '{}'", context.file_path))?;
		let mut archive = ZipArchive::new(BufReader::new(file))
			.with_context(|| format!("Failed to read ODT as zip '{}'", context.file_path))?;
		let content = read_zip_entry_by_name(&mut archive, "content.xml")
			.context("ODT file does not contain content.xml or it is empty")?;
		let xml_doc = XmlDocument::parse(&content).context("Invalid ODT content.xml")?;
		let mut buffer = DocumentBuffer::new();
		let mut id_positions = HashMap::new();
		traverse(xml_doc.root(), &mut buffer, &mut id_positions);
		let title =
			Path::new(&context.file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
		let toc_items = build_toc_from_buffer(&buffer);
		let mut document = Document::new().with_title(title);
		document.set_buffer(buffer);
		document.id_positions = id_positions;
		document.toc_items = toc_items;
		Ok(document)
	}
}

fn traverse(node: Node, buffer: &mut DocumentBuffer, id_positions: &mut HashMap<String, usize>) {
	if node.node_type() == NodeType::Element {
		let tag_name = node.tag_name().name();
		if tag_name == "h" {
			let level = node.attribute("outline-level").and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
			let heading_offset = buffer.current_position();
			let heading_text = get_element_text(node);
			if !heading_text.is_empty() {
				buffer.append(&heading_text);
				buffer.append("\n");
				let marker_type = match level {
					1 => MarkerType::Heading1,
					2 => MarkerType::Heading2,
					3 => MarkerType::Heading3,
					4 => MarkerType::Heading4,
					5 => MarkerType::Heading5,
					_ => MarkerType::Heading6,
				};
				buffer.add_marker(Marker::new(marker_type, heading_offset).with_text(heading_text).with_level(level));
			}
			return; // Don't traverse children, we already got the text
		}
		if tag_name == "p" {
			traverse_children(node, buffer, id_positions);
			buffer.append("\n");
			return;
		}
		if tag_name == "a" {
			if let Some(href) = node.attribute("href") {
				let link_offset = buffer.current_position();
				let link_text = get_element_text(node);
				if !link_text.is_empty() {
					buffer.append(&link_text);
					buffer.add_marker(
						Marker::new(MarkerType::Link, link_offset)
							.with_text(link_text)
							.with_reference(href.to_string()),
					);
				}
			}
			return; // Don't traverse children, we already got the text
		}
		if let Some(id) = node.attribute("id") {
			id_positions.insert(id.to_string(), buffer.current_position());
		}
	} else if node.node_type() == NodeType::Text {
		if let Some(text) = node.text() {
			buffer.append(text);
		}
		return;
	}
	traverse_children(node, buffer, id_positions);
}

fn traverse_children(node: Node, buffer: &mut DocumentBuffer, id_positions: &mut HashMap<String, usize>) {
	for child in node.children() {
		traverse(child, buffer, id_positions);
	}
}

fn get_element_text(node: Node) -> String {
	let mut text = String::new();
	collect_text(node, &mut text);
	text.trim().to_string()
}

fn collect_text(node: Node, text: &mut String) {
	if node.node_type() == NodeType::Text {
		if let Some(t) = node.text() {
			text.push_str(t);
		}
	}
	for child in node.children() {
		collect_text(child, text);
	}
}
