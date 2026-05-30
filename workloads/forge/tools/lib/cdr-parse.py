#!/usr/bin/env python3
"""Stage 2: Structural markdown parser for CDR pipeline.

Reads a SKILL.md file and outputs structured JSON, destroying all
raw markdown formatting. This is a security boundary — after parsing,
formatting-based attacks (hidden content, zero-width chars, etc.) are gone.

Usage: python3 cdr-parse.py <path-to-SKILL.md>
Output: JSON to stdout
Exit 1 on parse error.
"""
import json
import sys
import re


def parse_frontmatter(lines):
    """Extract YAML frontmatter between --- delimiters."""
    if not lines or lines[0].strip() != '---':
        return None, lines

    fm_lines = []
    for i, line in enumerate(lines[1:], 1):
        if line.strip() == '---':
            fm = {}
            for fl in fm_lines:
                fl = fl.strip()
                if ':' in fl:
                    key, _, val = fl.partition(':')
                    fm[key.strip()] = val.strip()
            return fm, lines[i + 1:]
        fm_lines.append(line)

    return None, lines


def parse_body(lines):
    """Parse markdown body into sections with prose and code blocks."""
    sections = []
    current_section = None
    in_code = False
    code_lang = ''
    code_lines = []

    for line in lines:
        stripped = line.strip()

        # Code fence toggle
        if stripped.startswith('```'):
            if in_code:
                # Closing fence
                if current_section is None:
                    current_section = {'heading': '', 'level': 0, 'prose': [], 'code_blocks': []}
                current_section['code_blocks'].append({
                    'language': code_lang,
                    'lines': code_lines
                })
                code_lines = []
                in_code = False
            else:
                # Opening fence
                code_lang = stripped[3:].strip()
                in_code = True
            continue

        if in_code:
            code_lines.append(line.rstrip())
            continue

        # Heading
        heading_match = re.match(r'^(#{1,6})\s+(.+)$', stripped)
        if heading_match:
            if current_section is not None:
                sections.append(current_section)
            current_section = {
                'heading': heading_match.group(2),
                'level': len(heading_match.group(1)),
                'prose': [],
                'code_blocks': []
            }
            continue

        # Prose (skip empty lines)
        if stripped and current_section is not None:
            current_section['prose'].append(stripped)
        elif stripped and current_section is None:
            current_section = {'heading': '', 'level': 0, 'prose': [stripped], 'code_blocks': []}

    if current_section is not None:
        sections.append(current_section)

    return sections


def main():
    if len(sys.argv) != 2:
        print('Usage: python3 cdr-parse.py <SKILL.md>', file=sys.stderr)
        sys.exit(1)

    filepath = sys.argv[1]
    try:
        with open(filepath, 'r') as f:
            lines = f.read().splitlines()
    except (FileNotFoundError, PermissionError) as e:
        print(f'Error: {e}', file=sys.stderr)
        sys.exit(1)

    if len(lines) < 3:
        print('Error: File too short to be a valid SKILL.md', file=sys.stderr)
        sys.exit(1)

    frontmatter, body_lines = parse_frontmatter(lines)
    if frontmatter is None:
        print('Error: No valid YAML frontmatter found', file=sys.stderr)
        sys.exit(1)

    sections = parse_body(body_lines)

    result = {
        'frontmatter': frontmatter,
        'sections': sections
    }

    json.dump(result, sys.stdout, indent=2)
    print()


if __name__ == '__main__':
    main()
