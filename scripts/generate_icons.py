"""Generate all public icon formats from the approved painterly artwork."""
from pathlib import Path
import base64
import hashlib
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
SOURCE = ASSETS / "logo.webp"
EXPECTED = "b05a60e219c3525d01dc37464972bc3f98162057dfa7f4fdf89f7b3c1608b848"

assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == EXPECTED, "Artwork checksum mismatch"
with Image.open(SOURCE) as original:
    original.load()
    assert original.size == (384, 384), "Unexpected icon dimensions"
    icon = original.convert("RGBA")
    icon.save(ASSETS / "logo.png", optimize=True)
    icon.save(ASSETS / "logo.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
encoded = base64.b64encode((ASSETS / "logo.png").read_bytes()).decode("ascii")
svg = (
    '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
    'width="384" height="384" viewBox="0 0 384 384" role="img" aria-label="PT Analytics Next">\n'
    '  <title>PT Analytics Next</title>\n'
    f'  <image width="384" height="384" href="data:image/png;base64,{encoded}" '
    f'xlink:href="data:image/png;base64,{encoded}"/>\n'
    '</svg>\n'
)
# The SVG is intentionally a raster-backed vector container, preserving the approved brushwork.
(ASSETS / "logo.svg").write_text(svg, encoding="utf-8", newline="\n")
(ROOT / "ui" / "logo.svg").write_text(svg, encoding="utf-8", newline="\n")

with Image.open(ASSETS / "logo.png") as png:
    assert png.size == (384,384) and png.mode == "RGBA"
with Image.open(ASSETS / "logo.ico") as ico:
    assert {(16,16),(32,32),(48,48),(256,256)} <= ico.ico.sizes()
print("Verified PNG, ICO and SVG icon assets.")
