#!/usr/bin/env python3
"""Stage 6: CDR reconstruction — builds fresh SKILL.md from intent JSON.

Every line of output is generated from structured data. No text is copied
from the original. Code blocks come from the cmd field of command objects.

Usage: python3 cdr-reconstruct.py <intent.json> <output-path>
"""
import json
import sys


def titlecase_slug(slug):
    """Convert 'docker-sandbox' to 'Docker Sandbox'."""
    return ' '.join(word.capitalize() for word in slug.split('-'))


def reconstruct(intent):
    lines = []

    name = intent['name']
    purpose = intent['purpose']

    # Frontmatter
    lines.append('---')
    lines.append(f'name: {name}')
    lines.append('version: 1.0.0')
    lines.append(f'description: {purpose}')
    lines.append('metadata: {}')
    lines.append('---')
    lines.append('')

    # Title
    lines.append(f'# {titlecase_slug(name)}')
    lines.append('')
    lines.append(purpose)
    lines.append('')

    # When to Use
    lines.append('## When to Use')
    lines.append('')
    for use_case in intent.get('use_cases', []):
        lines.append(f'- {use_case}')
    lines.append('')

    # Commands
    commands = intent.get('commands', [])
    if commands:
        lines.append('## Commands')
        lines.append('')
        for cmd_obj in commands:
            cmd = cmd_obj.get('cmd', '')
            context = cmd_obj.get('context', '')
            if context:
                lines.append(f'### {context}')
                lines.append('')
            lines.append('```bash')
            lines.append(cmd)
            lines.append('```')
            lines.append('')

    # Patterns
    patterns = intent.get('patterns', [])
    if patterns:
        lines.append('## Patterns')
        lines.append('')
        for pattern in patterns:
            title = pattern.get('title', '')
            desc = pattern.get('description', '')
            lines.append(f'### {title}')
            lines.append('')
            lines.append(desc)
            lines.append('')

    # Tips
    tips = intent.get('tips', [])
    if tips:
        lines.append('## Tips')
        lines.append('')
        for tip in tips:
            lines.append(f'- {tip}')
        lines.append('')

    return '\n'.join(lines)


def main():
    if len(sys.argv) != 3:
        print('Usage: python3 cdr-reconstruct.py <intent.json> <output-path>', file=sys.stderr)
        sys.exit(1)

    intent_path = sys.argv[1]
    output_path = sys.argv[2]

    try:
        with open(intent_path) as f:
            intent = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f'Error: {e}', file=sys.stderr)
        sys.exit(1)

    content = reconstruct(intent)

    with open(output_path, 'w') as f:
        f.write(content)

    print(f'Reconstructed: {output_path}')


if __name__ == '__main__':
    main()
