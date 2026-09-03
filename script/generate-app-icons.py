"""M4, second part: generate Clay's app icons from the 64x64 mark.

The mark is 64x64 pixel art, and every target size is an exact multiple or divisor of 64, so:

  * upscaling (256, 512, 1024) uses NEAREST, which keeps the hard pixel edges and matches the
    `shape-rendering="crispEdges"` in clay_icon.svg -- rasterising the SVG at those sizes would
    produce the same result;
  * downscaling (16, 32) uses LANCZOS, because nearest-neighbour throws away detail harshly at
    icon sizes where legibility matters more than crisp edges.

Deliberately not handled: Document.icns, because this Pillow build cannot write ICNS. Also, all
four release channels get the same mark, so they are no longer visually distinguishable the way
Zed's differently-coloured dev/preview/nightly icons were.
"""

import io
import os

from PIL import Image

SOURCE = "clay_icon.png"
RESOURCES = os.path.join("crates", "zed", "resources")
WINDOWS = os.path.join(RESOURCES, "windows")
CHANNELS = ["", "-dev", "-preview", "-nightly"]
ICO_SIZES = [16, 32, 64, 128, 256]


def scaled(base, size):
    if size == base.width:
        return base.copy()
    resample = Image.NEAREST if size > base.width else Image.LANCZOS
    return base.resize((size, size), resample)


def main():
    base = Image.open(SOURCE).convert("RGBA")
    assert base.size == (64, 64), base.size
    written = []

    for suffix in CHANNELS:
        png = os.path.join(RESOURCES, f"app-icon{suffix}.png")
        png2x = os.path.join(RESOURCES, f"app-icon{suffix}@2x.png")
        ico = os.path.join(WINDOWS, f"app-icon{suffix}.ico")

        scaled(base, 512).save(png)
        written.append(png)
        scaled(base, 1024).save(png2x)
        written.append(png2x)

        # Pillow builds the whole multi-size ICO from one image, downsampling internally, which
        # would blur the small entries. Compose it from per-size renders instead.
        largest = scaled(base, max(ICO_SIZES))
        buffers = [scaled(base, s) for s in ICO_SIZES]
        largest.save(ico, format="ICO", sizes=[(s, s) for s in ICO_SIZES], append_images=buffers)
        written.append(ico)

    for path in written:
        print(f"{os.path.getsize(path):>8}  {path}")


if __name__ == "__main__":
    main()
