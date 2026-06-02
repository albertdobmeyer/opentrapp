#!/usr/bin/env python3
"""Split skill content (stdin) into substantive prose paragraphs for the judge.

The static scan finds nothing in a clean-passing skill, so there is no specific
line to point the judge at — but judging the whole file at once DILUTES a single
buried natural-language instruction (a 3b model reads the file as "documentation"
and allows it). Judging each substantive paragraph instead keeps the signal sharp.

Emits one base64-encoded paragraph per line. Skips YAML frontmatter, markdown
headings, fenced code blocks, and trivially-short paragraphs; bounded to a max
number of chunks. (v0.6 Item D1.)
"""
import base64
import sys

MAX_CHUNKS = 15
MIN_LEN = 40


def chunks(text):
    lines = text.splitlines()
    # Strip YAML frontmatter (--- ... ---) at the top.
    if lines and lines[0].strip() == "---":
        end = next((i for i in range(1, len(lines)) if lines[i].strip() == "---"), None)
        if end is not None:
            lines = lines[end + 1:]

    out, buf, in_code = [], [], False
    for ln in lines:
        s = ln.strip()
        if s.startswith("```"):
            in_code = not in_code
            continue
        if in_code:
            continue
        if not s or s.startswith("#"):
            if buf:
                out.append(" ".join(buf))
                buf = []
            continue
        buf.append(s)
    if buf:
        out.append(" ".join(buf))

    return [p for p in out if len(p) >= MIN_LEN][:MAX_CHUNKS]


def main():
    for p in chunks(sys.stdin.read()):
        print(base64.b64encode(p.encode()).decode())


if __name__ == "__main__":
    main()
