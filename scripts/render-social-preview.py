"""Render the GitHub social-preview card for the OpenTrApp repo.

Produces a 1280x640 PNG composed of:
  - a diagonal slate gradient (slate-900 -> slate-800)
  - a faint dot grid for texture
  - the brand gradient FontLogo banner, centered slightly above midline
  - a tagline in muted slate
  - a monospace author footer

Output is written to docs/social-preview/opentrapp.png. Upload via
GitHub repo Settings -> General -> Social preview.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

REPO = Path(__file__).resolve().parent.parent
BANNER_SRC = REPO / "logos" / "OpenTrApp-FontLogo-Gradient.png"
OUT_PNG = REPO / "docs" / "social-preview" / "opentrapp.png"

W, H = 1280, 640
BG_TOP_LEFT = (15, 23, 42)        # slate-900 #0f172a
BG_BOTTOM_RIGHT = (30, 41, 59)    # slate-800 #1e293b
DOT_COLOR = (148, 163, 184)       # slate-400 — used at low alpha
TAGLINE_COLOR = (203, 213, 225)   # slate-300 #cbd5e1
FOOTER_COLOR = (100, 116, 139)    # slate-500 #64748b

TAGLINE = "A safer way to run OpenClaw on your own computer."
FOOTER = "albertdobmeyer  /  github.com/albertdobmeyer/opentrapp"

FONT_TAGLINE = "C:/Windows/Fonts/segoeui.ttf"
FONT_FOOTER = "C:/Windows/Fonts/consola.ttf"

BANNER_TARGET_W = 880  # source aspect 2000x609 -> 880x268


def render_background() -> Image.Image:
    """Diagonal gradient slate-900 -> slate-800 across the canvas."""
    bg = Image.new("RGB", (W, H), BG_TOP_LEFT)
    px = bg.load()
    diag = (W - 1) + (H - 1)
    for y in range(H):
        for x in range(W):
            t = (x + y) / diag
            px[x, y] = (
                round(BG_TOP_LEFT[0] + (BG_BOTTOM_RIGHT[0] - BG_TOP_LEFT[0]) * t),
                round(BG_TOP_LEFT[1] + (BG_BOTTOM_RIGHT[1] - BG_TOP_LEFT[1]) * t),
                round(BG_TOP_LEFT[2] + (BG_BOTTOM_RIGHT[2] - BG_TOP_LEFT[2]) * t),
            )
    return bg


def overlay_dots(img: Image.Image) -> None:
    """Faint dot grid at low alpha for subtle texture."""
    dots = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(dots)
    spacing = 32
    for y in range(spacing, H, spacing):
        for x in range(spacing, W, spacing):
            draw.ellipse((x - 1, y - 1, x + 1, y + 1), fill=(*DOT_COLOR, 24))
    img.alpha_composite(dots)


def paste_banner_with_glow(canvas: Image.Image) -> tuple[int, int, int, int]:
    """Place the gradient banner with a soft brand-coloured glow behind it.

    Returns the bounding box of the placed banner so callers can stack
    captions underneath without overlap.
    """
    banner = Image.open(BANNER_SRC).convert("RGBA")
    src_w, src_h = banner.size
    target_h = round(BANNER_TARGET_W * src_h / src_w)
    banner = banner.resize((BANNER_TARGET_W, target_h), Image.LANCZOS)

    bx = (W - BANNER_TARGET_W) // 2
    by = 150  # leaves ~270px below the banner for tagline + footer

    # Brand glow: take the banner's alpha as a mask, paint it green-blue,
    # blur it heavily, and lay it under the banner.
    alpha = banner.split()[-1]
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    glow_layer = Image.new("RGBA", banner.size, (0, 0, 0, 0))
    gx_w, gx_h = banner.size
    glow_px = glow_layer.load()
    alpha_px = alpha.load()
    for y in range(gx_h):
        ty = y / max(1, gx_h - 1)
        for x in range(gx_w):
            a = alpha_px[x, y]
            if not a:
                continue
            tx = x / max(1, gx_w - 1)
            t = (tx + ty) / 2
            r = round(0 + (14 - 0) * t)        # green -> blue R
            g = round(153 + (165 - 153) * t)    # green -> blue G
            b = round(102 + (233 - 102) * t)    # green -> blue B
            glow_px[x, y] = (r, g, b, min(255, a))
    glow_layer = glow_layer.filter(ImageFilter.GaussianBlur(radius=42))
    glow.alpha_composite(glow_layer, dest=(bx, by))
    # Lift the glow opacity globally so it reads against the slate.
    r, g, b, a = glow.split()
    a = a.point(lambda v: min(255, int(v * 0.85)))
    glow = Image.merge("RGBA", (r, g, b, a))
    canvas.alpha_composite(glow)
    canvas.alpha_composite(banner, dest=(bx, by))
    return bx, by, bx + BANNER_TARGET_W, by + target_h


def draw_centered(draw: ImageDraw.ImageDraw, text: str, font: ImageFont.FreeTypeFont, y: int, fill: tuple[int, int, int]) -> None:
    bbox = draw.textbbox((0, 0), text, font=font)
    text_w = bbox[2] - bbox[0]
    draw.text(((W - text_w) // 2, y), text, font=font, fill=fill)


def main() -> None:
    canvas = render_background().convert("RGBA")
    overlay_dots(canvas)
    _, _, _, banner_bottom = paste_banner_with_glow(canvas)

    draw = ImageDraw.Draw(canvas)
    tagline_font = ImageFont.truetype(FONT_TAGLINE, 30)
    footer_font = ImageFont.truetype(FONT_FOOTER, 18)

    draw_centered(draw, TAGLINE, tagline_font, banner_bottom + 36, TAGLINE_COLOR)
    draw_centered(draw, FOOTER, footer_font, H - 56, FOOTER_COLOR)

    OUT_PNG.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(OUT_PNG, "PNG", optimize=True)
    print(f"wrote {OUT_PNG} ({W}x{H})")


if __name__ == "__main__":
    main()
