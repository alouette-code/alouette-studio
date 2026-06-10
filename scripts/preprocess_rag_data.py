#!/usr/bin/env python3
"""Preprocess snippet_samples.csv → clean JSON for RAG seeding.

Reads the large CSV, deduplicates, filters noise, and outputs
a structured JSON file that seed.rs can load.
"""

import csv
import json
import os
import re
import sys

csv.field_size_limit(10_000_000)

INPUT = os.path.join(
    os.path.dirname(__file__),
    "..",
    "core_engine",
    "app_data",
    "db_RAG",
    "snippet_samples.csv",
)
OUTPUT = os.path.join(
    os.path.dirname(__file__),
    "..",
    "core_engine",
    "app_data",
    "db_RAG",
    "seed_library.json",
)

MAX_PER_LANG = 0  # 0 = không giới hạn
MIN_SNIPPET_LEN = 10  # Chỉ bỏ snippet quá ngắn (<10 ký tự)

# Map tên ngôn ngữ trong CSV → lang_id chuẩn
LANG_MAP = {
    "Python": "python",
    "Java": "java",
    "JavaScript": "javascript",
    "C++": "cpp",
    "C": "c",
    "C#": "csharp",
    "PHP": "php",
    "Go": "go",
    "Ruby": "ruby",
    "TypeScript": "typescript",
    "Bash": "sh",
    "Shell": "sh",
    "Rust": "rust",
    "HTML": "html",
    "CSV": "csv",
    "Markdown": "markdown",
}


def clean_snippet(text: str) -> str:
    """Làm sạch snippet: trim, bỏ khoảng trắng thừa."""
    lines = text.split("\n")
    # Bỏ dòng đầu/cuối trống
    while lines and lines[0].strip() == "":
        lines.pop(0)
    while lines and lines[-1].strip() == "":
        lines.pop()
    return "\n".join(lines)


def extract_name(snippet: str, lang: str) -> str:
    """Trích xuất tên hàm từ snippet."""
    patterns = [
        r"(?:def|fn|function|func|sub)\s+(\w+)",
        r"(\w+)\s*\([^)]*\)\s*(?:\{|:|->|{)",
        r"class\s+(\w+)",
        r"public\s+(?:static\s+)?\w+\s+(\w+)\s*\(",
        r"(?:int|void|string|bool|float|double|char)\s+(\w+)\s*\(",
    ]
    first_line = snippet.split("\n")[0]
    for p in patterns:
        m = re.search(p, first_line)
        if m:
            return m.group(1)
    # Fallback: lấy chữ đầu tiên
    words = re.findall(r"\w+", first_line)
    return words[0] if words else "unknown"


def main():
    counts = {}
    entries = []

    with open(INPUT, "r", encoding="utf-8", errors="ignore") as f:
        reader = csv.DictReader(f)
        for row in reader:
            lang_raw = row.get("language", "").strip()
            snippet_raw = row.get("snippet", "").strip()
            lang_id = LANG_MAP.get(lang_raw)

            if not lang_id or not snippet_raw:
                continue
            if lang_id not in (
                "python",
                "java",
                "javascript",
                "cpp",
                "c",
                "rust",
                "sh",
            ):
                continue
            if len(snippet_raw) < MIN_SNIPPET_LEN:
                continue

            # Giới hạn mỗi ngôn ngữ (nếu MAX_PER_LANG > 0)
            if MAX_PER_LANG > 0 and counts.get(lang_id, 0) >= MAX_PER_LANG:
                continue

            snippet = clean_snippet(snippet_raw)
            first_line = snippet.split("\n")[0] if snippet else ""
            if not first_line:
                continue

            name = extract_name(snippet, lang_id)
            if name == "unknown":
                continue

            # Tạo docstring ngắn từ dòng đầu
            docstring = first_line[:120] if first_line else None

            entry = {
                "lang_id": lang_id,
                "func_name": name,
                "signature": first_line[:200],
                "docstring": docstring,
                "snippet": snippet[:2000],
                "file_path": f"library/{lang_id}/{name}",
            }
            entries.append(entry)
            counts[lang_id] = counts.get(lang_id, 0) + 1

    with open(OUTPUT, "w", encoding="utf-8") as f:
        json.dump(entries, f, indent=2, ensure_ascii=False)

    print(f"✅ Done: {len(entries)} entries → {OUTPUT}")
    for lang, c in sorted(counts.items(), key=lambda x: -x[1]):
        print(f"   {lang}: {c}")


if __name__ == "__main__":
    main()
