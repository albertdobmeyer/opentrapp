#!/usr/bin/env python3
"""Stage 5: CDR intent schema validation.

Validates intent JSON against the expected schema.
Exit 0 if valid, exit 1 with error message if invalid.

Usage: python3 cdr-validate.py <intent.json>
"""
import json
import re
import sys


def validate(intent):
    errors = []

    # Required fields
    for field in ('name', 'purpose', 'use_cases', 'commands', 'tips'):
        if field not in intent:
            errors.append(f'Missing required field: {field}')

    if errors:
        return errors

    # name: valid slug
    name = intent['name']
    if not isinstance(name, str) or not re.match(r'^[a-z0-9][a-z0-9-]*$', name):
        errors.append(f'Invalid name "{name}": must be lowercase slug (letters, numbers, hyphens)')

    # purpose: non-empty string
    purpose = intent['purpose']
    if not isinstance(purpose, str) or len(purpose) < 10:
        errors.append('purpose must be a string of at least 10 characters')

    # use_cases: non-empty array of strings
    use_cases = intent['use_cases']
    if not isinstance(use_cases, list) or len(use_cases) < 1:
        errors.append('use_cases must be a non-empty array')
    elif not all(isinstance(u, str) for u in use_cases):
        errors.append('use_cases must contain only strings')

    # commands: array of {cmd, context}
    commands = intent['commands']
    if not isinstance(commands, list):
        errors.append('commands must be an array')
    else:
        for i, cmd in enumerate(commands):
            if not isinstance(cmd, dict):
                errors.append(f'commands[{i}] must be an object')
            elif 'cmd' not in cmd or 'context' not in cmd:
                errors.append(f'commands[{i}] must have "cmd" and "context" fields')

    # tips: non-empty array of strings
    tips = intent['tips']
    if not isinstance(tips, list) or len(tips) < 1:
        errors.append('tips must be a non-empty array')
    elif not all(isinstance(t, str) for t in tips):
        errors.append('tips must contain only strings')

    # patterns: optional array of {title, description}
    patterns = intent.get('patterns', [])
    if not isinstance(patterns, list):
        errors.append('patterns must be an array')

    # Field length limits (prevent bloat injection)
    for key, val in intent.items():
        if isinstance(val, str) and len(val) > 1000:
            errors.append(f'Field "{key}" exceeds 1000 character limit ({len(val)} chars)')
        elif isinstance(val, list):
            for i, item in enumerate(val):
                if isinstance(item, str) and len(item) > 1000:
                    errors.append(f'{key}[{i}] exceeds 1000 character limit')
                elif isinstance(item, dict):
                    for k, v in item.items():
                        if isinstance(v, str) and len(v) > 1000:
                            errors.append(f'{key}[{i}].{k} exceeds 1000 character limit')

    return errors


def main():
    if len(sys.argv) != 2:
        print('Usage: python3 cdr-validate.py <intent.json>', file=sys.stderr)
        sys.exit(1)

    try:
        with open(sys.argv[1]) as f:
            intent = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f'Error: {e}', file=sys.stderr)
        sys.exit(1)

    errors = validate(intent)
    if errors:
        for e in errors:
            print(f'INVALID: {e}', file=sys.stderr)
        sys.exit(1)

    print('VALID')


if __name__ == '__main__':
    main()
