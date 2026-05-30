---
name: test-clean
version: 1.0.0
description: A clean test skill for CDR validation.
metadata: {"clawdbot":{"emoji":"✅"}}
---

# Test Clean Skill

A simple skill for testing the CDR pipeline.

## When to Use

- When you need to validate CDR produces a clean reconstruction
- When testing the end-to-end pipeline

## Commands

```bash
echo "hello world"
ls -la /tmp
grep -r "pattern" .
```

## Tips

- Always test with clean fixtures first
- Verify the reconstruction matches the intent
- Check that quarantine is cleaned up
