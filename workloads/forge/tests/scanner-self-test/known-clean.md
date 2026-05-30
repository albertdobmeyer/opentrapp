---
name: known-clean
version: 0.0.0
description: Perfectly clean skill for scanner self-test
---

# Known Clean Skill

This file should produce zero findings.

## When to Use

Use this when you need a baseline clean skill for testing.

## How It Works

```bash
echo "Hello, world"
ls -la /tmp
grep -r "pattern" ./src
```

## Tips

- Always validate your inputs
- Use shellcheck for bash scripts
- Keep dependencies minimal

```python
def greet(name: str) -> str:
    return f"Hello, {name}"
```
