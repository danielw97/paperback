use std::{collections::HashMap, fs};

use anyhow::{Context, Result};
use roxmltree::{Document as XmlDocument, Node, NodeType};

use crate::{
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags},
	parser::Parser,
	xml_to_text::XmlToText,
};

pub struct Fb2Parser;

impl Parser for Fb2Parser {
	fn name(&self) -> &str {
		"FictionBook Documents"
	}

	fn extensions(&self) -> &[&str] {
		&["fb2"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_TOC | ParserFlags::SUPPORTS_SECTIONS
	}

	fn parse(&self, context: &ParserContext) -> Result<Document> {
		let mut xml_content = fs::read_to_string(&context.file_path)
			.with_context(|| format!("Failed to read FB2 file '{}'", context.file_path))?;
		if xml_content.is_empty() {
			anyhow::bail!("FB2 file is empty");
		}
		const CLOSING_TAG: &str = "</FictionBook>";
		if let Some(pos) = xml_content.rfind(CLOSING_TAG) {
			xml_content.truncate(pos + CLOSING_TAG.len());
		}
		xml_content = remove_binary_elements(&xml_content).unwrap_or(xml_content);
		let (title, author) = extract_metadata(&xml_content);
		let mut converter = XmlToText::new();
		if !converter.convert(&xml_content) {
			anyhow::bail!("Failed to convert FB2 XML to text");
		}
		let mut buffer = DocumentBuffer::new();
		buffer.append(&converter.get_text());
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
		for offset in converter.get_section_offsets() {
			buffer.add_marker(Marker::new(MarkerType::SectionBreak, *offset));
		}
		for link in converter.get_links() {
			buffer.add_marker(
				Marker::new(MarkerType::Link, link.offset)
					.with_text(link.text.clone())
					.with_reference(link.reference.clone()),
			);
		}
		let id_positions: HashMap<String, usize> = converter.get_id_positions().clone();
		let mut document = Document::new().with_title(title).with_author(author);
		document.set_buffer(buffer);
		document.id_positions = id_positions;
		Ok(document)
	}
}

fn remove_binary_elements(xml_content: &str) -> Option<String> {
	let doc = XmlDocument::parse(xml_content).ok()?;
	let mut result = String::new();
	serialize_without_binary(doc.root(), &mut result);
	Some(result)
}

fn serialize_without_binary(node: Node, output: &mut String) {
	match node.node_type() {
		NodeType::Root => {
			for child in node.children() {
				serialize_without_binary(child, output);
			}
		}
		NodeType::Element => {
			let tag_name = node.tag_name().name();
			if tag_name == "binary" {
				return;
			}
			output.push('<');
			output.push_str(tag_name);
			for attr in node.attributes() {
				output.push(' ');
				output.push_str(attr.name());
				output.push_str("=\"");
				output.push_str(&escape_xml(attr.value()));
				output.push('"');
			}
			if node.children().count() == 0 {
				output.push_str("/>");
			} else {
				output.push('>');
				for child in node.children() {
					serialize_without_binary(child, output);
				}
				output.push_str("</");
				output.push_str(tag_name);
				output.push('>');
			}
		}
		NodeType::Text => {
			if let Some(text) = node.text() {
				output.push_str(&escape_xml(text));
			}
		}
		NodeType::Comment => {
			if let Some(text) = node.text() {
				output.push_str("<!--");
				output.push_str(text);
				output.push_str("-->");
			}
		}
		NodeType::PI => {
			if let Some(text) = node.text() {
				output.push_str("<?");
				output.push_str(text);
				output.push_str("?>");
			}
		}
	}
}

fn escape_xml(s: &str) -> String {
	s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn extract_metadata(xml_content: &str) -> (String, String) {
	let doc = match XmlDocument::parse(xml_content) {
		Ok(d) => d,
		Err(_) => return (String::new(), String::new()),
	};
	let mut title = String::new();
	let mut author = String::new();
	if let Some(title_node) =
		find_element_by_path(doc.root(), &["FictionBook", "description", "title-info", "book-title"])
	{
		title = get_element_text_content(title_node).trim().to_string();
	}
	if let Some(author_node) = find_element_by_path(doc.root(), &["FictionBook", "description", "title-info", "author"]) {
		let first_name =
			find_child_by_name(author_node, "first-name").map(get_element_text_content).unwrap_or_default();
		let last_name = find_child_by_name(author_node, "last-name").map(get_element_text_content).unwrap_or_default();
		if !first_name.is_empty() {
			author.push_str(&first_name);
		}
		if !last_name.is_empty() {
			if !author.is_empty() {
				author.push(' ');
			}
			author.push_str(&last_name);
		}
		author = author.trim().to_string();
	}
	(title, author)
}

fn find_element_by_path<'a, 'input>(node: Node<'a, 'input>, path: &[&str]) -> Option<Node<'a, 'input>> {
	if path.is_empty() {
		return Some(node);
	}
	let target = path[0];
	let remaining = &path[1..];
	for child in node.children() {
		if child.node_type() == NodeType::Element {
			let tag_name = child.tag_name().name();
			if tag_name == target {
				if remaining.is_empty() {
					return Some(child);
				}
				return find_element_by_path(child, remaining);
			}
		}
	}
	None
}

fn find_child_by_name<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
	for child in node.children() {
		if child.node_type() == NodeType::Element && child.tag_name().name() == name {
			return Some(child);
		}
	}
	None
}

fn get_element_text_content(node: Node) -> String {
	let mut text = String::new();
	collect_text_content(node, &mut text);
	text
}

fn collect_text_content(node: Node, text: &mut String) {
	if node.node_type() == NodeType::Text {
		if let Some(t) = node.text() {
			text.push_str(t);
		}
	}
	for child in node.children() {
		collect_text_content(child, text);
	}
}
