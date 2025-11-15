/* rust_parser.hpp - adapter for Rust-based parsers.
 *
 * Paperback.
 * Copyright (c) 2025 Quin Gillespie.
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

#pragma once
#include "parser.hpp"
#include <memory>
#include <span>
#include <string>
#include <vector>
#include <wx/string.h>

class rust_parser : public parser {
public:
	rust_parser(wxString parser_name, std::vector<wxString> exts, parser_flags flags);
	[[nodiscard]] wxString name() const override;
	[[nodiscard]] std::span<const wxString> extensions() const override;
	[[nodiscard]] std::unique_ptr<document> load(const parser_context& ctx) const override;
	[[nodiscard]] parser_flags supported_flags() const override;

private:
	wxString parser_name_;
	std::vector<wxString> extensions_;
	parser_flags flags_;
};

class rust_html_parser final : public rust_parser {
public:
	rust_html_parser();
};

class rust_markdown_parser final : public rust_parser {
public:
	rust_markdown_parser();
};

class rust_text_parser final : public rust_parser {
public:
	rust_text_parser();
};

REGISTER_PARSER(rust_html_parser)
REGISTER_PARSER(rust_markdown_parser)
REGISTER_PARSER(rust_text_parser)
