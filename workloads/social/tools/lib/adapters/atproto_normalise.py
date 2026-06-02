#!/usr/bin/env python3
"""Normalise an AT Protocol feed response to the canonical post shape.

Reads a `app.bsky.feed.getAuthorFeed` / `getFeed` XRPC response object (or a raw
list of feedViewPosts) on stdin, writes a JSON array of
`{id, author, content, timestamp}` on stdout.

Pure + network-free so the adapter's normalisation is unit-testable against a
fixture (the atproto adapter factors this out; the Moltbook adapter inlines its
equivalent). Malformed input → an empty array (never a crash).
"""
import json
import sys


def normalise(raw):
    if isinstance(raw, list):
        items = raw
    elif isinstance(raw, dict):
        items = raw.get("feed", raw.get("posts", []))
    else:
        items = []

    out = []
    for item in items:
        if not isinstance(item, dict):
            continue
        # A feedViewPost wraps the actual post under "post"; tolerate a bare post.
        post = item.get("post", item)
        if not isinstance(post, dict):
            continue
        author = post.get("author") or {}
        record = post.get("record") or {}
        out.append({
            "id": post.get("uri", ""),
            "author": author.get("handle", author.get("did", "unknown")),
            "content": record.get("text", ""),
            "timestamp": record.get("createdAt", post.get("indexedAt", "")),
        })
    return out


def main():
    try:
        raw = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        print("[]")
        return
    print(json.dumps(normalise(raw)))


if __name__ == "__main__":
    main()
