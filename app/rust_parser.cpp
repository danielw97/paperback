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
		const std::string file_path = ctx.file_path.ToStdString();
		const std::string password = ctx.password.value_or("");
		const auto ffi_doc = parse_document(rust::Str(file_path), rust::Str(password));
		auto doc = std::make_unique<document>();
		doc->title = wxString::FromUTF8(std::string(ffi_doc.title).c_str());
		doc->author = wxString::FromUTF8(std::string(ffi_doc.author).c_str());
		doc->buffer.set_content(wxString::FromUTF8(std::string(ffi_doc.content).c_str()));
		for (const auto& rust_marker : ffi_doc.markers) {
			const auto marker_type_value = static_cast<marker_type>(rust_marker.marker_type);
			const wxString text = wxString::FromUTF8(std::string(rust_marker.text).c_str());
			const wxString ref = wxString::FromUTF8(std::string(rust_marker.reference).c_str());
			doc->buffer.add_marker(rust_marker.position, marker_type_value, text, ref, rust_marker.level);
		}
		doc->buffer.finalize_markers();
		// Note: For now, we're adding them as flat items
		// A more sophisticated approach would reconstruct the hierarchy
		for (const auto& rust_toc : ffi_doc.toc_items) {
			auto item = std::make_unique<toc_item>();
			item->name = wxString::FromUTF8(std::string(rust_toc.name).c_str());
			item->ref = wxString::FromUTF8(std::string(rust_toc.reference).c_str());
			item->offset = rust_toc.offset;
			doc->toc_items.push_back(std::move(item));
		}
		doc->stats.word_count = ffi_doc.stats.word_count;
		doc->stats.line_count = ffi_doc.stats.line_count;
		doc->stats.char_count = ffi_doc.stats.char_count;
		return doc;
	} catch (const std::exception& e) {
		throw parser_exception(wxString::FromUTF8(e.what()), ctx.file_path);
	}
}

rust_html_parser::rust_html_parser() : rust_parser("HTML Documents", {"htm", "html", "xhtml"}, parser_flags::supports_toc | parser_flags::supports_lists) {
}

rust_markdown_parser::rust_markdown_parser() : rust_parser("Markdown Files", {"md", "markdown", "mdown", "mkdn", "mkd"}, parser_flags::supports_toc) {
}

rust_text_parser::rust_text_parser() : rust_parser("Text Files", {"txt", "log"}, parser_flags::none) {
}
