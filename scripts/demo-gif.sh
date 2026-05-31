#!/usr/bin/env bash
# Convert Playwright screenshot slideshows into docs-ready gifs.
#
# Usage:
#   scripts/demo-gif.sh                    # run all demo specs + convert
#   scripts/demo-gif.sh demo-tour.spec.ts  # run a single named spec
#
# Why a slideshow and not Playwright's built-in video recorder?
#   The video pipeline produces blank frames on this Linux setup even though
#   page.screenshot() captures the DOM correctly (verified live). The
#   slideshow approach is also higher-quality for docs gifs — each frame is
#   a clean PNG, no VP8 compression artefacts in the gif palette.
#
# Pipeline:
#   1. Run Playwright in the `demo` project (each spec writes
#      test-results/<dir>/frames/NNN.png + delays.txt).
#   2. For each test-results/* dir with frames/, run ImageMagick `convert`
#      with the per-frame delays from delays.txt.
#   3. Drop the gifs at docs/assets/<spec-name>.gif at the repo root.
#
# Outputs:
#   docs/assets/demo-wizard.gif
#   docs/assets/demo-tour.gif
#
# Requirements: ImageMagick (`convert` command).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${REPO_ROOT}/app"
ASSETS_DIR="${REPO_ROOT}/docs/assets"

if ! command -v convert >/dev/null 2>&1; then
    echo "ERROR: ImageMagick 'convert' not found in PATH." >&2
    echo "       Install imagemagick and try again." >&2
    exit 2
fi
if [ ! -f "${APP_DIR}/playwright.config.ts" ]; then
    echo "ERROR: ${APP_DIR}/playwright.config.ts not found." >&2
    exit 2
fi

mkdir -p "${ASSETS_DIR}"

SPEC_FILTER="${1:-}"

# Clean prior demo run output so the find below only matches the fresh recording.
rm -rf "${APP_DIR}/test-results"

echo "==> Recording demo with Playwright (project=demo)"
cd "${APP_DIR}"
if [ -n "${SPEC_FILTER}" ]; then
    npx playwright test --project=demo "${SPEC_FILTER}"
else
    npx playwright test --project=demo
fi

echo ""
echo "==> Assembling gifs from screenshot slideshows"

shopt -s nullglob
count=0
for delays_file in "${APP_DIR}/test-results"/*/delays.txt; do
    test_dir="$(dirname "${delays_file}")"
    frames_dir="${test_dir}/frames"
    if [ ! -d "${frames_dir}" ]; then
        echo "  WARN: ${delays_file} exists but no frames/ — skipping" >&2
        continue
    fi

    dir_name="$(basename "${test_dir}")"
    case "${dir_name}" in
        *demo-wizard*) gif_name="demo-wizard.gif" ;;
        *demo-tour*)   gif_name="demo-tour.gif" ;;
        *)             gif_name="${dir_name%-demo*}.gif" ;;
    esac
    output="${ASSETS_DIR}/${gif_name}"

    # Build convert args from delays.txt. Each line is "NNN.png <centiseconds>".
    # We pass -delay before each -file so each frame gets its own hold time.
    args=()
    while IFS=' ' read -r frame_name centi; do
        [ -z "${frame_name}" ] && continue
        args+=("-delay" "${centi}" "${frames_dir}/${frame_name}")
    done < "${delays_file}"

    if [ "${#args[@]}" -eq 0 ]; then
        echo "  WARN: ${delays_file} is empty — skipping ${dir_name}" >&2
        continue
    fi

    echo "  ${dir_name} → ${output} ($((${#args[@]} / 3)) frames)"

    # -layers Optimize merges identical regions across frames to shrink the
    # gif. -loop 0 = loop forever.
    convert "${args[@]}" -loop 0 -layers Optimize "${output}"

    size_kb=$(( $(stat -c '%s' "${output}" 2>/dev/null || stat -f '%z' "${output}") / 1024 ))
    echo "    ${size_kb} KB"
    count=$((count + 1))
done
shopt -u nullglob

if [ "${count}" -eq 0 ]; then
    echo "WARN: no demo slideshow output found under ${APP_DIR}/test-results/" >&2
    echo "      Did the playwright run produce frames? Check the run log above." >&2
    exit 1
fi

echo ""
echo "Done. ${count} gif(s) at ${ASSETS_DIR}/"
ls -lh "${ASSETS_DIR}"/*.gif
