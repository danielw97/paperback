/* utils.cpp - miscellaneous helpers shared across Paperback.
 *
 * Paperback.
 * Copyright (c) 2025 Quin Gillespie.
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

#include "utils.hpp"
#include "app.hpp"
#include "config_manager.hpp"
#include "constants.hpp"
#include "dialogs.hpp"
#include "libpaperback/src/bridge.rs.h"
#include "live_region.hpp"
#include "main_window.hpp"
#include "parser.hpp"
#include <cctype>
#include <cstddef>
#include <iterator>
#include <optional>
#include <regex>
#include <sstream>
#include <string>
#include <string_view>
#include <wx/defs.h>
#include <wx/strconv.h>
#include <wx/string.h>

namespace {
constexpr unsigned char UTF8_NBSP_FIRST = 0xC2;
constexpr unsigned char UTF8_NBSP_SECOND = 0xA0;

long find_text_regex(const wxString& haystack, const wxString& needle, long start, find_options options) {
	const auto forward = has_option(options, find_options::forward);
	const auto match_case = has_option(options, find_options::match_case);
	const auto match_whole_word = has_option(options, find_options::match_whole_word);
	try {
		auto pattern = needle.ToStdString();
		const auto text = haystack.ToStdString();
		if (match_whole_word) {
			pattern = "\\b" + pattern + "\\b";
		}
		std::regex_constants::syntax_option_type flags = std::regex_constants::ECMAScript;
		if (!match_case) {
			flags |= std::regex_constants::icase;
		}
		const std::regex rx(pattern, flags);
		if (forward) {
			std::cmatch m;
			const char* begin = text.c_str() + std::min<size_t>(static_cast<size_t>(start), text.size());
			if (std::regex_search(begin, text.c_str() + text.size(), m, rx)) {
				return static_cast<long>(m[0].first - text.c_str());
			}
		} else {
			long last_match = wxNOT_FOUND;
			std::cmatch m;
			const char* base = text.c_str();
			const size_t end = std::min<size_t>(static_cast<size_t>(start), text.size());
			const char* cur = base;
			while (cur <= base + end) {
				if (std::regex_search(cur, base + end, m, rx)) {
					last_match = static_cast<long>(m[0].first - base);
					cur = m[0].first + 1;
				} else {
					break;
				}
			}
			return last_match;
		}
	} catch (...) {
		return wxNOT_FOUND;
	}
	return wxNOT_FOUND;
}

long find_text_literal(const wxString& haystack, const wxString& needle, long start, find_options options) {
	const auto forward = has_option(options, find_options::forward);
	const auto match_case = has_option(options, find_options::match_case);
	const auto match_whole_word = has_option(options, find_options::match_whole_word);
	const auto& search_haystack = match_case ? haystack : haystack.Lower();
	const auto& search_needle = match_case ? needle : needle.Lower();
	if (!match_whole_word) {
		return forward ? static_cast<long>(search_haystack.find(search_needle, static_cast<size_t>(start))) : static_cast<long>(search_haystack.Left(start).rfind(search_needle));
	}
	long pos = start;
	while (true) {
		pos = forward ? static_cast<long>(search_haystack.find(search_needle, static_cast<size_t>(pos))) : static_cast<long>(search_haystack.Left(static_cast<size_t>(pos)).rfind(search_needle));
		if (pos == static_cast<long>(wxNOT_FOUND)) {
			break;
		}
		const bool word_start = (pos == 0) || (wxIsalnum(haystack[static_cast<size_t>(pos) - 1]) == 0);
		const bool word_end = (static_cast<size_t>(pos) + needle.length() >= haystack.length()) || (wxIsalnum(haystack[static_cast<size_t>(pos) + needle.length()]) == 0);
		if (word_start && word_end) {
			return pos;
		}
		pos = forward ? pos + 1 : pos - 1;
		if (forward && static_cast<size_t>(pos) >= haystack.length()) {
			break;
		}
		if (!forward && pos < 0) {
			break;
		}
	}
	return wxNOT_FOUND;
}
} // namespace

long find_text(const wxString& haystack, const wxString& needle, long start, find_options options) {
	if (needle.empty()) {
		return wxNOT_FOUND;
	}
	if (has_option(options, find_options::use_regex)) {
		return find_text_regex(haystack, needle, start, options);
	}
	return find_text_literal(haystack, needle, start, options);
}

std::string collapse_whitespace(std::string_view input) {
	try {
		std::string input_str(input);
		return std::string(::collapse_whitespace(rust::Str(input_str)));
	} catch (const std::exception&) {
		return {};
	}
}

std::string trim_string(const std::string& str) {
	try {
		return std::string(::trim_string(rust::Str(str)));
	} catch (const std::exception&) {
		return {};
	}
}

std::string remove_soft_hyphens(std::string_view input) {
	try {
		std::string input_str(input);
		return std::string(::remove_soft_hyphens(rust::Str(input_str)));
	} catch (const std::exception&) {
		return {};
	}
}

const parser* get_parser_for_unknown_file(const wxString& path, config_manager& config) {
	const wxString saved_format = config.get_document_format(path);
	if (!saved_format.IsEmpty()) {
		const auto* par = find_parser_by_extension(saved_format);
		if (par != nullptr) {
			return par;
		}
	}
	open_as_dialog dlg(nullptr, path);
	if (dlg.ShowModal() != wxID_OK) {
		return nullptr;
	}
	const wxString format = dlg.get_selected_format();
	config.set_document_format(path, format);
	return find_parser_by_extension(format);
}

void speak(const wxString& message) {
	auto* main_win = dynamic_cast<main_window*>(wxGetApp().GetTopWindow());
	if (main_win == nullptr) {
		return;
	}
	auto* label = main_win->get_live_region_label();
	if (label == nullptr) {
		return;
	}
	label->SetLabel(message);
	notify_live_region_changed(label);
}

std::string url_decode(std::string_view encoded) {
	try {
		std::string encoded_str(encoded);
		return std::string(::url_decode(rust::Str(encoded_str)));
	} catch (const std::exception&) {
		return {};
	}
}

std::string convert_to_utf8(const std::string& input) {
	if (input.empty()) {
		return input;
	}
	try {
		rust::Slice<const uint8_t> slice(reinterpret_cast<const uint8_t*>(input.data()), input.length());
		return std::string(::convert_to_utf8(slice));
	} catch (const std::exception&) {
		return input;
	}
}

void cleanup_toc(std::vector<std::unique_ptr<toc_item>>& items) {
	for (auto& item : items) {
		if (!item->children.empty()) {
			auto& first_child = item->children[0];
			if (item->name.CmpNoCase(first_child->name) == 0 && (item->ref == first_child->ref || item->ref.IsEmpty())) {
				if (item->ref.IsEmpty() && !first_child->ref.IsEmpty()) {
					item->ref = first_child->ref;
					item->offset = first_child->offset;
				}
				auto grandchildren = std::move(first_child->children);
				item->children.erase(item->children.begin());
				item->children.insert(item->children.begin(), std::make_move_iterator(grandchildren.begin()), std::make_move_iterator(grandchildren.end()));
			}
		}
		cleanup_toc(item->children);
	}
}

std::vector<std::unique_ptr<toc_item>> build_toc_from_headings(const document_buffer& buffer) {
	std::vector<std::unique_ptr<toc_item>> result;
	const auto heading_markers = buffer.get_heading_markers();
	if (heading_markers.empty()) {
		return result;
	}
	std::vector<std::vector<std::unique_ptr<toc_item>>*> level_stacks(MAX_HEADING_LEVELS + 1, nullptr);
	level_stacks[0] = &result;
	for (const auto* marker : heading_markers) {
		auto item = std::make_unique<toc_item>();
		item->name = marker->text;
		item->offset = static_cast<int>(marker->pos);
		const int level = marker->level;
		if (level < 1 || level > MAX_HEADING_LEVELS) {
			continue;
		}
		std::vector<std::unique_ptr<toc_item>>* parent_list = nullptr;
		for (int i = level - 1; i >= 0; --i) {
			if (level_stacks[i] != nullptr) {
				parent_list = level_stacks[i];
				break;
			}
		}
		if (parent_list == nullptr) {
			parent_list = &result;
		}
		parent_list->push_back(std::move(item));
		level_stacks[level] = &parent_list->back()->children;
		for (int i = level + 1; i < MAX_HEADING_LEVELS + 1; ++i) {
			level_stacks[i] = nullptr;
		}
	}
	return result;
}

std::string read_zip_entry(const std::string& zip_path, const std::string& entry_name) {
	try {
		return std::string(::read_zip_entry(rust::Str(zip_path), rust::Str(entry_name)));
	} catch (const std::exception&) {
		return {};
	}
}

size_t find_zip_entry(const std::string& zip_path, const std::string& entry_name) {
	try {
		return ::find_zip_entry(rust::Str(zip_path), rust::Str(entry_name));
	} catch (const std::exception&) {
		return static_cast<size_t>(-1);
	}
}
