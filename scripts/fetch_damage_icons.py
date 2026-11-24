"""Download Warframe's original text icons; no synthesized shapes or runtime requests."""
import hashlib
import io
import json
from pathlib import Path
import time
from urllib.request import Request, urlopen
from PIL import Image

root = Path(__file__).resolve().parents[1]
sources = json.loads((root / "assets/damage-icons.json").read_text())
out = root / "ui/damage-icons"
out.mkdir(parents=True, exist_ok=True)

def validate(data):
    with Image.open(io.BytesIO(data)) as image:
        if image.format != "PNG" or not (1 <= image.width <= 1024 and 1 <= image.height <= 1024):
            raise ValueError("Expected a small PNG image")
        image.verify()
    with Image.open(io.BytesIO(data)) as image:
        rgba = image.convert("RGBA")
        low, high = rgba.getchannel("A").getextrema()
        if low == high or high == 0:
            raise ValueError("Expected a visible icon with transparent background")

checksums = {}
for name, url in sources.items():
    path = out / (name + ".png")
    data = None
    if path.is_file():
        try:
            data = path.read_bytes()
            validate(data)
        except (OSError, ValueError):
            data = None
    if data is None:
        for attempt in range(3):
            try:
                with urlopen(Request(url, headers={"User-Agent": "PT-Analytics-Next/0.1.7 (build-time asset download)"}), timeout=25) as response:
                    data = response.read(2 * 1024 * 1024 + 1)
                if len(data) > 2 * 1024 * 1024:
                    raise ValueError("Icon response too large")
                validate(data)
                temp = path.with_suffix(".tmp")
                temp.write_bytes(data)
                temp.replace(path)
                break
            except Exception as exc:
                if attempt == 2:
                    raise RuntimeError(f"Failed to fetch {name}: {url}") from exc
                time.sleep(2 ** attempt)
    checksums[name] = {"source": url, "sha256": hashlib.sha256(data).hexdigest()}
    print(f"Verified {name}: {len(data)} bytes")
(out / "manifest.json").write_text(json.dumps(checksums, indent=2) + "\n", encoding="utf-8")
print(f"Verified all {len(sources)} original Warframe icons.")
