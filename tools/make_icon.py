"""Generates assets/fileflip.ico and assets/fileflip.png (run: python tools/make_icon.py).

Glossy 3D illustration: a fanned stack of documents (red, blue, white) with a faceted
green arrow in front. 16-24 px use a simplified, high-contrast variant.
"""
from PIL import Image, ImageChops, ImageDraw, ImageFilter

S = 1024
PW, PH, FOLD = 500, 640, 150  # page size and folded-corner size


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(len(a)))


def gradient(size, c1, c2, diag=0.4):
    w, h = size
    g = Image.new("RGBA", size)
    px = g.load()
    for y in range(h):
        for x in range(w):
            t = max(0.0, min(1.0, (y + x * diag) / (h + w * diag)))
            px[x, y] = lerp(c1, c2, t) + (255,)
    return g


def page_mask(w, h, fold, r=46):
    m = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle((0, 0, w - 1, h - 1), radius=r, fill=255)
    d.polygon([(w - fold, -2), (w + 2, -2), (w + 2, fold)], fill=0)
    return m


def page(c1, c2, flap_color, lines=None):
    """One document as an RGBA layer (with margin for rotation and shadow)."""
    m = 80
    layer = Image.new("RGBA", (PW + 2 * m, PH + 2 * m), (0, 0, 0, 0))
    mask = Image.new("L", layer.size, 0)
    mask.paste(page_mask(PW, PH, FOLD), (m, m))
    body = Image.new("RGBA", layer.size, (0, 0, 0, 0))
    body.paste(gradient((PW, PH), c1, c2), (m, m))
    layer.paste(body, (0, 0), mask)

    # darker rim for a rounded, solid edge
    inner = mask.filter(ImageFilter.MinFilter(15)).filter(ImageFilter.GaussianBlur(6))
    rim = ImageChops.subtract(mask, inner)
    dark = Image.new("RGBA", layer.size, lerp(c2, (0, 0, 0), 0.35) + (0,))
    dark.putalpha(rim.point(lambda v: v * 3 // 5))
    layer.alpha_composite(dark)

    # glossy highlight across the top
    gloss = Image.new("L", layer.size, 0)
    ImageDraw.Draw(gloss).ellipse((m - PW * 0.3, m - PH * 0.55, m + PW * 1.1, m + PH * 0.38), fill=90)
    gloss = ImageChops.multiply(gloss.filter(ImageFilter.GaussianBlur(30)), inner)
    white = Image.new("RGBA", layer.size, (255, 255, 255, 0))
    white.putalpha(gloss)
    layer.alpha_composite(white)

    d = ImageDraw.Draw(layer)
    if lines:
        for i, wfrac in enumerate([0.62, 0.78, 0.7, 0.8, 0.55]):
            y = m + 230 + i * 70
            d.rounded_rectangle((m + 70, y, m + 70 + int((PW - 140) * wfrac), y + 26), radius=13, fill=lines)

    # folded corner: shadow under the flap, then the flap
    fs = Image.new("L", layer.size, 0)
    ImageDraw.Draw(fs).polygon([(m + PW - FOLD, m), (m + PW - FOLD - 10, m + FOLD + 20), (m + PW, m + FOLD)], fill=150)
    fs = ImageChops.multiply(fs.filter(ImageFilter.GaussianBlur(14)), mask)
    sh = Image.new("RGBA", layer.size, (0, 0, 0, 0))
    sh.putalpha(fs)
    layer.alpha_composite(sh)
    flap = Image.new("L", layer.size, 0)
    ImageDraw.Draw(flap).polygon([(m + PW - FOLD, m), (m + PW - FOLD, m + FOLD - 18), (m + PW - FOLD + 18, m + FOLD), (m + PW, m + FOLD)], fill=255)
    layer.paste(gradient(layer.size, lerp(flap_color, (255, 255, 255), 0.6), flap_color, diag=0.0), (0, 0), flap)
    return layer


def drop_shadow(layer, offset=(10, 26), blur=22, alpha=120):
    a = layer.getchannel("A").point(lambda v: v * alpha // 255)
    sh = Image.new("RGBA", layer.size, (20, 20, 40, 0))
    sh.putalpha(a)
    out = Image.new("RGBA", (layer.width + 120, layer.height + 120), (0, 0, 0, 0))
    out.alpha_composite(sh.filter(ImageFilter.GaussianBlur(blur)), (60 + offset[0], 60 + offset[1]))
    out.alpha_composite(layer, (60, 60))
    return out


def place(canvas, layer, cx, cy, angle):
    rot = drop_shadow(layer).rotate(angle, resample=Image.BICUBIC, expand=True)
    canvas.alpha_composite(rot, (int(cx - rot.width / 2), int(cy - rot.height / 2)))


def arrow(canvas, scale=1.0, shadow=True):
    """IDM-style faceted triangle pointing right, bottom-right of the icon."""
    def p(x, y):
        return (x * scale, y * scale)
    A, T, B = p(520, 545), p(985, 735), p(585, 1000)
    C = ((A[0] + T[0] + B[0]) / 3 - 18 * scale, (A[1] + T[1] + B[1]) / 3 - 22 * scale)
    if shadow:
        m = Image.new("L", canvas.size, 0)
        ImageDraw.Draw(m).polygon([A, T, B], fill=150)
        sh = Image.new("RGBA", canvas.size, (10, 40, 10, 0))
        sh.putalpha(m.filter(ImageFilter.GaussianBlur(18)))
        canvas.alpha_composite(sh.transform(canvas.size, Image.AFFINE, (1, 0, -8, 0, 1, -18)))
    d = ImageDraw.Draw(canvas)
    d.polygon([A, T, C], fill=(132, 225, 72, 255))   # top facet, lit
    d.polygon([B, A, C], fill=(64, 176, 46, 255))    # left facet
    d.polygon([T, B, C], fill=(22, 112, 34, 255))    # bottom facet, shade
    d.line([A, T], fill=(200, 255, 160, 255), width=int(8 * scale))
    d.line([A, C, T], fill=(170, 240, 120, 180), width=max(1, int(4 * scale)))


def render_3d():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    place(img, page((248, 113, 113), (185, 28, 28), (254, 202, 202)), 360, 440, 14)
    place(img, page((96, 165, 250), (30, 64, 175), (191, 219, 254)), 600, 420, -10)
    place(img, page((255, 226, 120), (240, 165, 30), (255, 244, 200), lines=(37, 99, 235, 255)), 470, 520, 2)
    arrow(img)
    return img


def render_small(size):
    k = 4
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((300, 110, 820, 820), radius=60, fill=(37, 99, 235, 255))
    front = page_mask(560, 760, 190, r=60)
    img.paste(Image.new("RGBA", front.size, (252, 200, 60, 255)), (150, 160), front)
    d.polygon([(520, 160), (710, 350), (520, 350)], fill=(255, 238, 170, 255))
    d.rounded_rectangle((150, 160, 709, 919), radius=60, outline=(29, 78, 216, 255), width=34)
    arrow(img, shadow=False)
    return img.resize((size * k, size * k), Image.LANCZOS).resize((size, size), Image.LANCZOS)


big = render_3d()
sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
frames = [render_small(s) if s <= 24 else big.resize((s, s), Image.LANCZOS) for s in sizes]
frames[-1].save("assets/fileflip.ico", format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[:-1])
frames[-1].save("assets/fileflip.png")

prev = Image.new("RGBA", (2 * 430, 270), (255, 255, 255, 255))
ImageDraw.Draw(prev).rectangle((430, 0, 860, 270), fill=(32, 32, 32, 255))
for half in (0, 1):
    x = 10 + half * 430
    for f in [frames[0], frames[2], frames[3], frames[5], frames[-1]]:
        prev.alpha_composite(f, (x, 6))
        x += f.width + 10
prev.save("icon-preview.png")
print("ok")
