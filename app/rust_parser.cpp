/* rust_parser.cpp - adapter for Rust-based parsers.
 *
 * Paperback.
 * Copyright (c) 2025 Quin Gillespie.
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

#include "rust_parser.hpp"
#include "document.hpp"
#include "document_buffer.hpp"
#include "libpaperback/src/bridge.rs.h"
#include <memory>
#include <stdexcept>
#include <wx/translation.h>

namespace {
wxString to_wxstring(const rust::String& rust_str) {
	return wxString::FromUTF8(std::string(rust_str).c_str());
}

void populate_markers(document_buffer& buffer, const rust::Vec<FfiMarker>& ffi_markers) {
	for (const auto& rust_marker : ffi_markers) {
		const auto marker_type_value = static_cast<marker_type>(rust_marker.marker_type);
		const wxString text = to_wxstring(rust_marker.text);
		const wxString ref = to_wxstring(rust_marker.reference);
		buffer.add_marker(rust_marker.position, marker_type_value, text, ref, rust_marker.level);
	}
	buffer.finalize_markers();
}

void populate_toc_items(std::vector<std::unique_ptr<toc_item>>& toc_items, const rust::Vec<FfiTocItem>& ffi_toc_items) {
	if (ffi_toc_items.empty()) {
		return;
	}
	constexpr int MAX_DEPTH = 32;
	std::vector<std::vector<std::unique_ptr<toc_item>>*> depth_stacks(MAX_DEPTH + 1, nullptr);
	depth_stacks[0] = &toc_items;
	for (const auto& rust_toc : ffi_toc_items) {
		auto item = std::make_unique<toc_item>();
		item->name = to_wxstring(rust_toc.name);
		item->ref = to_wxstring(rust_toc.reference);
		item->offset = rust_toc.offset;
		const int depth = rust_toc.depth;
		if (depth < 0 || depth > MAX_DEPTH) {
			continue;
		}
		std::vector<std::unique_ptr<toc_item>>* parent_list = nullptr;
		for (int i = depth; i >= 0; --i) {
			if (depth_stacks[i] != nullptr) {
				parent_list = depth_stacks[i];
				break;
			}
		}
		if (parent_list == nullptr) {
			parent_list = &toc_items;
		}
		parent_list->push_back(std::move(item));
		depth_stacks[depth + 1] = &parent_list->back()->children;
		for (int i = depth + 2; i <= MAX_DEPTH; ++i) {
			depth_stacks[i] = nullptr;
		}
	}
}

void populate_stats(document_stats& stats, const FfiDocumentStats& ffi_stats) {
	stats.word_count = ffi_stats.word_count;
	stats.line_count = ffi_stats.line_count;
	stats.char_count = ffi_stats.char_count;
}

void populate_id_positions(document& doc, const rust::Vec<FfiIdPosition>& ffi_positions) {
	doc.id_positions.clear();
	for (const auto& entry : ffi_positions) {
		doc.id_positions[std::string(entry.id)] = entry.offset;
	}
}

void populate_spine_items(document& doc, const rust::Vec<rust::String>& ffi_spine_items) {
	doc.spine_items.clear();
	for (const auto& item : ffi_spine_items) {
		doc.spine_items.emplace_back(std::string(item));
	}
}

void populate_manifest_items(document& doc, const rust::Vec<FfiManifestItem>& ffi_manifest) {
	doc.manifest_items.clear();
	for (const auto& entry : ffi_manifest) {
		doc.manifest_items[std::string(entry.id)] = std::string(entry.path);
	}
}
} // anonymous namespace

rust_parser::rust_parser(wxString parser_name, std::vector<wxString> exts, parser_flags flags) : parser_name_{std::move(parser_name)}, extensions_{std::move(exts)}, flags_{flags} {
}

wxString rust_parser::name() const {
	return parser_name_;
}

std::span<const wxString> rust_parser::extensions() const {
	return extensions_;
}

parser_flags rust_parser::supported_flags() const {
	return flags_;
}

std::unique_ptr<document> rust_parser::load(const parser_context& ctx) const {
	try {
		const std::string file_path = ctx.file_path.ToUTF8().data();
		const std::string password = ctx.password.value_or("");
		const auto ffi_doc = parse_document(rust::Str(file_path), rust::Str(password));
		auto doc = std::make_unique<document>();
		doc->title = to_wxstring(ffi_doc.title);
		doc->author = to_wxstring(ffi_doc.author);
		doc->buffer.set_content(to_wxstring(ffi_doc.content));
		populate_markers(doc->buffer, ffi_doc.markers);
		populate_toc_items(doc->toc_items, ffi_doc.toc_items);
		populate_stats(doc->stats, ffi_doc.stats);
		populate_id_positions(*doc, ffi_doc.id_positions);
		populate_spine_items(*doc, ffi_doc.spine_items);
		populate_manifest_items(*doc, ffi_doc.manifest_items);
		return doc;
	} catch (const std::exception& e) {
		throw parser_exception(wxString::FromUTF8(e.what()), ctx.file_path);
	}
}

rust_docx_parser::rust_docx_parser() : rust_parser("Word Documents", {"docx", "docm"}, parser_flags::supports_toc) {
}

rust_epub_parser::rust_epub_parser() : rust_parser("Epub Books", {"epub"}, parser_flags::supports_sections | parser_flags::supports_toc | parser_flags::supports_lists) {
}

rust_fb2_parser::rust_fb2_parser() : rust_parser("FictionBook Documents", {"fb2"}, parser_flags::supports_toc | parser_flags::supports_sections) {
}

rust_html_parser::rust_html_parser() : rust_parser("HTML Documents", {"htm", "html", "xhtml"}, parser_flags::supports_toc | parser_flags::supports_lists) {
}

rust_markdown_parser::rust_markdown_parser() : rust_parser("Markdown Files", {"md", "markdown", "mdown", "mkdn", "mkd"}, parser_flags::supports_toc) {
}

rust_odp_parser::rust_odp_parser() : rust_parser("OpenDocument Presentations", {"odp"}, parser_flags::none) {
}

rust_odt_parser::rust_odt_parser() : rust_parser("OpenDocument Text Files", {"odt"}, parser_flags::supports_toc) {
}

rust_pptx_parser::rust_pptx_parser() : rust_parser("PowerPoint Presentations", {"pptx", "pptm"}, parser_flags::supports_toc) {
}

rust_text_parser::rust_text_parser() : rust_parser("Text Files", {"txt", "log"}, parser_flags::none) {
}
