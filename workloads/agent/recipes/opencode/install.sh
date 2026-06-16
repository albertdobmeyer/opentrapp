#!/bin/sh
# Agent recipe: opencode — SCAFFOLD (not yet implemented; fail-closed by design).
#
# A correct opencode recipe needs opencode's VERIFIED install + launch details (how it is
# installed, its config path, and that it is a TTY session — not a Telegram bot — so this
# recipe will DROP the Telegram gateway/waker). The opencode reconnaissance could not be
# trusted on specifics (org name, issue numbers), so this recipe intentionally fails the
# build rather than shipping a guessed install. See recipes/README.md and task #11
# (agent-recipe part 2b). Unblock by filling in the verified install below.
set -eu
echo "ERROR: the 'opencode' agent recipe is not yet implemented." >&2
echo "It is blocked on verified opencode install/launch details — see recipes/README.md." >&2
exit 1
