"""
parser.py — Sprint 3 upgrade.

Previously returned a single flat string.  Now returns a list of structured
ParsedBlock objects so heading hierarchy and page numbers survive into the
Rust chunker.

Protocol: reads JSON lines from stdin, writes one JSON line per request to stdout.

Input:  {"file_path": "/path/to/file.pdf"}
Output (success):
  {
    "status": "success",
    "blocks": [
      {
        "heading_path": ["Chapter 1", "Introduction"],
        "page_number": 1,          // null for formats with no page concept
        "text": "...",
        "ocr_confidence": null     // null — native text extraction
      },
      ...
    ],
    // Legacy flat-text field kept for backward compat with old callers.
    "content": "..."
  }
Output (error):
  {"status": "error", "error": "...", "traceback": "..."}
"""

import sys
import json
import re
import traceback


# ---------------------------------------------------------------------------
# Heading + page-break parsing helpers
# ---------------------------------------------------------------------------

# MarkItDown emits standard ATX headings: # H1 / ## H2 / ### H3 …
_HEADING_RE = re.compile(r'^(#{1,6})\s+(.+)$')

# MarkItDown inserts page-break markers like:
#   <!-- Page Break -->   or   <!-- PageBreak -->   or   ---
# We also look for the explicit marker it adds for PDFs:
#   \f   (form-feed character)  and   "--- Page N ---" patterns.
_PAGE_BREAK_RE = re.compile(
    r'<!--\s*[Pp]age\s*[Bb]reak\s*-->|'
    r'<!--\s*[Pp]agebreak\s*-->|'
    r'\f|'
    r'^---\s*[Pp]age\s*\d+\s*---$',
    re.MULTILINE,
)


def _parse_markdown_into_blocks(markdown_text: str) -> list[dict]:
    """
    Walk the Markdown line by line and group lines into ParsedBlock dicts.

    Each block spans from one heading (or the document start) to the next
    heading or page-break marker.  Page numbers are tracked by counting
    page-break markers (1-indexed: starts at 1, increments on each break).
    """
    lines = markdown_text.splitlines(keepends=True)

    blocks: list[dict] = []
    heading_stack: list[str] = []   # current breadcrumb
    current_page = 1
    current_lines: list[str] = []
    block_page = 1

    def flush_block(hpath: list[str], page: int, text_lines: list[str]) -> None:
        text = "".join(text_lines).strip()
        if text:
            blocks.append({
                "heading_path": list(hpath),
                "page_number": page,
                "text": text,
                "ocr_confidence": None,
            })

    for raw_line in lines:
        line = raw_line.rstrip('\n').rstrip('\r')

        # --- Check for page break ---
        if _PAGE_BREAK_RE.search(line) or '\f' in raw_line:
            flush_block(heading_stack, block_page, current_lines)
            current_lines = []
            current_page += 1
            block_page = current_page
            continue

        # --- Check for ATX heading ---
        m = _HEADING_RE.match(line)
        if m:
            flush_block(heading_stack, block_page, current_lines)
            current_lines = []
            level = len(m.group(1))         # number of '#' chars
            heading_text = m.group(2).strip()

            # Trim stack to current level then push new heading
            heading_stack = heading_stack[:level - 1]
            heading_stack.append(heading_text)
            block_page = current_page
            continue

        current_lines.append(raw_line)

    # Flush final block
    flush_block(heading_stack, block_page, current_lines)

    return blocks


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------

def main():
    try:
        from markitdown import MarkItDown
        md = MarkItDown()
    except ImportError:
        print(json.dumps({
            "status": "error",
            "error": "markitdown not installed — run: pip install markitdown"
        }), flush=True)
        sys.exit(1)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            req = json.loads(line)
            file_path = req.get("file_path")

            if not file_path:
                print(json.dumps({
                    "status": "error",
                    "error": "file_path is required"
                }), flush=True)
                continue

            result = md.convert(file_path)
            flat_content = result.text_content or ""

            blocks = _parse_markdown_into_blocks(flat_content)

            print(json.dumps({
                "status": "success",
                "blocks": blocks,
                # Legacy field — kept so old callers that only read "content"
                # don't break.  Will be removed after Sprint 5 refactor.
                "content": flat_content,
            }), flush=True)

        except Exception as e:
            err = traceback.format_exc()
            print(json.dumps({
                "status": "error",
                "error": str(e),
                "traceback": err
            }), flush=True)


if __name__ == "__main__":
    main()
