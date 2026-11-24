"""Package the exact checked-out sources and generated build inputs, excluding build caches."""
import argparse
from pathlib import Path
import zipfile

p = argparse.ArgumentParser()
p.add_argument("stage", type=Path)
p.add_argument("tracked_manifest", type=Path)
p.add_argument("output", type=Path)
a = p.parse_args()
stage = a.stage.resolve()
tracked = set(a.tracked_manifest.read_bytes().decode("utf-8").split("\0"))
tracked.discard("")
generated = {"package-lock.json", "src-tauri/Cargo.lock", "assets/logo.png",
             "assets/logo.ico", "assets/logo.svg", "ui/logo.svg"}
for directory in ("src-tauri/icons", "ui/damage-icons"):
    generated.update(x.relative_to(stage).as_posix() for x in (stage / directory).rglob("*") if x.is_file())
required = {"package-lock.json", "src-tauri/Cargo.lock", "ui/damage-icons/manifest.json",
            "src-tauri/icons/icon.ico"}
if not all((stage / f).is_file() for f in required):
    raise SystemExit("Generated assets or lock files missing")
a.output.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(a.output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as z:
    for rel in sorted(tracked | generated):
        src = (stage / rel).resolve()
        if not src.is_relative_to(stage) or not src.is_file():
            raise SystemExit(f"Missing or invalid source: {rel}")
        z.write(src, rel)
with zipfile.ZipFile(a.output) as z:
    assert z.testzip() is None, "Invalid source ZIP"
print(f"Source archive: {a.output}")
