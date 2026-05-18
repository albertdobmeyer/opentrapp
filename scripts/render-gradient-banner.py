"""Render a brand-gradient version of the OpenTrApp FontLogo banner.

Replaces the flat brand-green pill with a 135° linear gradient running from
OpenTrApp-Green (#009966) at the top-left to OpenTrApp-Blue (#0EA5E9)
at the bottom-right. Preserves the white outline + text and the red shield.
"""

from pathlib import Path

from PIL import Image

REPO = Path(__file__).resolve().parent.parent
SOURCE = REPO / "logos" / "red-green-logos" / "OpenTrApp-FontLogo-Light-RedGreen.png"
TARGETS = [
    REPO / "app" / "public" / "logo-banner.png",
    REPO / "docs" / "img" / "logo-banner.png",
    REPO / "logos" / "OpenTrApp-FontLogo-Gradient.png",
]

GREEN_PILL = (0, 153, 102)  # exact source colour to replace
START = (0, 153, 102)        # OpenTrApp-Green
END = (14, 165, 233)         # OpenTrApp-Blue


def lerp(a: int, b: int, t: float) -> int:
    return round(a + (b - a) * t)


def main() -> None:
    src = Image.open(SOURCE).convert("RGBA")
    pixels = src.load()
    w, h = src.size
    denom = (w - 1) + (h - 1)

    for y in range(h):
        for x in range(w):
            r, g, b, a = pixels[x, y]
            if (r, g, b) == GREEN_PILL and a == 255:
                t = (x + y) / denom
                pixels[x, y] = (
                    lerp(START[0], END[0], t),
                    lerp(START[1], END[1], t),
                    lerp(START[2], END[2], t),
                    255,
                )

    for target in TARGETS:
        target.parent.mkdir(parents=True, exist_ok=True)
        src.save(target, "PNG", optimize=True)
        print(f"wrote {target}")


if __name__ == "__main__":
    main()
