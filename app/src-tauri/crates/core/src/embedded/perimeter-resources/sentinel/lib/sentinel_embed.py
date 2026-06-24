#!/usr/bin/env python3
"""Sentinel rung-1 — local embeddings for similarity / anomaly / drift.

The cheap, always-affordable layer between rung 0 (static regex) and rung 2
(the tiny LLM judge). It never decides on its own authority beyond "this is
clearly like known-bad" / "this is clearly unlike anything bad" — the ambiguous
middle band is handed up to rung 2. See docs/specs/v0.6/01-sentinel-spine.md §4.

Engine: a small sentence-embedding model (default all-MiniLM-L6-v2, ~45 MB,
384-dim, Apache-2.0) served by the SAME local Ollama the rung-2 judge and the
CDR parser already use — no new runtime, no API key, zero marginal cost (the
v0.6 anti-bloat contract). D2 resolved: all-minilm.

Three uses (spec 01 §4):
  1. similarity — how close is a fragment to a corpus of known-bad examples?
  2. anomaly    — (same mechanism) distance from the caller's normal traffic.
  3. drift      — distance between an outgoing post and the agent's recent
                  voice + task, to catch a hijacked agent posting off-character.

Subcommands (all read the text to judge on stdin unless noted):
  vector                       stdin text -> {"model","dim","vector":[...]}
  build-corpus OUT FIELD IN... read JSON arrays IN..., embed each item[FIELD],
                               write the cached corpus to OUT (a build step;
                               re-running is a cheap re-embed, never a retrain)
  score CORPUS                 stdin fragment -> {"max_similarity","nearest_ref",
                               "signal":"suspicious|ambiguous|clean"}
  drift HISTORY [TASK_HINT]    stdin post -> {"similarity","drift",
                               "signal":"in_character|drifted"}

Thresholds are env-driven (model-dependent — tuned empirically, see the test
suite). Exit 3 if Ollama / the embed model is unreachable so the caller can
choose its fail-closed/open policy (mirrors judge.sh's exit-2 contract).
"""
import json
import math
import os
import sys
import urllib.request

ENDPOINT = os.environ.get("SENTINEL_EMBED_ENDPOINT", "http://localhost:11434/api/embed")
MODEL = os.environ.get("SENTINEL_EMBED_MODEL", "all-minilm")
TIMEOUT = int(os.environ.get("SENTINEL_EMBED_TIMEOUT", "30"))

# Similarity bands for `score` (cosine vs the known-bad corpus). Calibrated for
# all-MiniLM-L6-v2 against the social/skills fixtures (see embed.test.sh):
# a near-duplicate of a known-bad example lands >= SIM_HIGH; genuinely unrelated
# benign content <= SIM_LOW; the middle band is ambiguous.
#
# RECALL CAVEAT (banked finding — see the module header and the spec): against a
# small known-bad corpus, embedding similarity reliably fires on NEAR-DUPLICATES
# but MISSES novel paraphrases (a hand-written exfil paraphrase scored only 0.32
# in calibration). So `score` is a recall-SAFE BOOSTER, not a gate: "suspicious"
# is a strong positive signal a caller can act on, but a "clean"/"ambiguous"
# result must NOT suppress the rung-2 judge for injection detection — low
# similarity is not evidence of safety. Callers use `score` to PRIORITISE and to
# escalate-early, never to skip rung 2 on its say-so.
SIM_HIGH = float(os.environ.get("SENTINEL_SIM_HIGH", "0.70"))
SIM_LOW = float(os.environ.get("SENTINEL_SIM_LOW", "0.30"))

# Drift floor for `drift` (max cosine of an outgoing post vs the agent's recent
# posts + task). This is the RELIABLE rung-1 signal: comparing against the
# agent's OWN voice is specific, where similarity-to-generic-bad is weak. In
# calibration, hijacked/off-character posts sat at ~0.11-0.15 and in-character
# posts at ~0.38-0.55, so 0.25 separates them with margin. Below this the post
# reads as off-character and is held for the user (a hold-for-review, not a
# block — an off-topic but benign post is correctly surfaced for one tap).
DRIFT_SIM_MIN = float(os.environ.get("SENTINEL_DRIFT_SIM_MIN", "0.25"))


def _fail(reason, code=3):
    print(json.dumps({"error": reason}))
    sys.exit(code)


def embed(text):
    """Return the embedding vector for `text` via local Ollama."""
    if not text or not text.strip():
        return None
    payload = json.dumps({"model": MODEL, "input": text}).encode()
    req = urllib.request.Request(
        ENDPOINT, data=payload, headers={"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            data = json.loads(resp.read())
    except Exception as e:  # noqa: BLE001 — any failure is "engine unreachable"
        _fail("The local similarity engine could not be reached (%s)." % type(e).__name__)
    vecs = data.get("embeddings") or ([data["embedding"]] if "embedding" in data else [])
    if not vecs:
        _fail("The local similarity engine returned no vector.")
    return vecs[0]


def cosine(a, b):
    if not a or not b or len(a) != len(b):
        return 0.0
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(y * y for y in b))
    if na == 0 or nb == 0:
        return 0.0
    return dot / (na * nb)


def _centroid(vectors):
    if not vectors:
        return None
    dim = len(vectors[0])
    return [sum(v[i] for v in vectors) / len(vectors) for i in range(dim)]


def _iter_field(obj, field):
    """Yield item[field] strings from a JSON array (or a single object)."""
    items = obj if isinstance(obj, list) else [obj]
    for it in items:
        if isinstance(it, dict) and isinstance(it.get(field), str):
            yield it["id"] if isinstance(it.get("id"), str) else "", it[field]
        elif isinstance(it, str):
            yield "", it


def cmd_vector():
    text = sys.stdin.read()
    v = embed(text)
    print(json.dumps({"model": MODEL, "dim": len(v), "vector": v}))


def cmd_build_corpus(out, field, inputs):
    examples = []
    for path in inputs:
        with open(path, encoding="utf-8") as fh:
            obj = json.load(fh)
        for ref, text in _iter_field(obj, field):
            v = embed(text)
            label = (text[:60] + "…") if len(text) > 60 else text
            examples.append({"ref": ref or label, "label": label, "vector": v})
    corpus = {"model": MODEL, "field": field, "count": len(examples), "examples": examples}
    with open(out, "w", encoding="utf-8") as fh:
        json.dump(corpus, fh)
    print(json.dumps({"written": out, "count": len(examples), "model": MODEL}))


def cmd_score(corpus_path):
    fragment = sys.stdin.read()
    fv = embed(fragment)
    with open(corpus_path, encoding="utf-8") as fh:
        corpus = json.load(fh)
    if corpus.get("model") and corpus["model"] != MODEL:
        _fail("Corpus was built with a different model (%s); rebuild it." % corpus["model"])
    best_sim, best_ref = 0.0, ""
    for ex in corpus.get("examples", []):
        s = cosine(fv, ex["vector"])
        if s > best_sim:
            best_sim, best_ref = s, ex.get("ref", "")
    if best_sim >= SIM_HIGH:
        signal = "suspicious"
    elif best_sim <= SIM_LOW:
        signal = "clean"
    else:
        signal = "ambiguous"
    print(json.dumps({
        "max_similarity": round(best_sim, 4),
        "nearest_ref": best_ref,
        "signal": signal,
    }))


def cmd_drift(history_path, task_hint=""):
    post = sys.stdin.read()
    pv = embed(post)
    with open(history_path, encoding="utf-8") as fh:
        obj = json.load(fh)
    anchors = []
    # The history file is the agent's own recent posts (same fixture shape).
    for _ref, text in _iter_field(obj, "content"):
        v = embed(text)
        if v:
            anchors.append(v)
    # The task the agent is supposed to be doing is an extra voice anchor.
    if task_hint.strip():
        tv = embed(task_hint)
        if tv:
            anchors.append(tv)
    if not anchors:
        _fail("No history to compare against — cannot judge drift.")
    # MAX over individual anchors (kNN-style), not the centroid: "is this post
    # like the agent's CLOSEST recent post / its task?". The centroid of a
    # diverse post history averages to a weak generic vector that washes out the
    # signal; the nearest-anchor distance separates in-character from hijacked
    # far more robustly (see embed.test.sh calibration).
    sim = max(cosine(pv, a) for a in anchors)
    signal = "drifted" if sim < DRIFT_SIM_MIN else "in_character"
    print(json.dumps({
        "similarity": round(sim, 4),
        "drift": round(1.0 - sim, 4),
        "signal": signal,
    }))


def main():
    if len(sys.argv) < 2:
        _fail("usage: sentinel_embed.py {vector|build-corpus|score|drift} ...", code=2)
    cmd = sys.argv[1]
    if cmd == "vector":
        cmd_vector()
    elif cmd == "build-corpus":
        if len(sys.argv) < 5:
            _fail("usage: build-corpus OUT FIELD IN...", code=2)
        cmd_build_corpus(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif cmd == "score":
        if len(sys.argv) < 3:
            _fail("usage: score CORPUS", code=2)
        cmd_score(sys.argv[2])
    elif cmd == "drift":
        if len(sys.argv) < 3:
            _fail("usage: drift HISTORY [TASK_HINT]", code=2)
        cmd_drift(sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else "")
    else:
        _fail("unknown subcommand: %s" % cmd, code=2)


if __name__ == "__main__":
    main()
