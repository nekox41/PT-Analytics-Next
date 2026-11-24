#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail
cd -- "$(dirname -- "$0")"
ROOT="$PWD"
REPO=nekox41/PT-Analytics-Next
TARGET_BRANCH=master
publish=1
bootstrap=0
check_only=0
usage() {
    cat <<'EOF'
Usage: bash build-release.sh [--build-only] [--bootstrap] [--check-only]
  Default: build Windows x64 portable EXE + NSIS, test, create GitHub Release.
  --build-only  Do not publish, only place binaries/source/checksums in dist/.
  --bootstrap   Install missing Windows tools (winget optional) and missing WSL gh via apt.
  --check-only  Check PowerShell syntax only; no installs, builds or publishing.
  --help        Show this message.
Build runs on native Windows through WSL interop, not on GitHub Actions.
EOF
}
while (($#)); do
    case "$1" in
        --build-only) publish=0 ;;
        --bootstrap) bootstrap=1 ;;
        --check-only) check_only=1; publish=0 ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
    shift
done
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
grep -qi microsoft /proc/sys/kernel/osrelease || die 'Run this script inside WSL2.'
for cmd in git tar python3 sha256sum wslpath powershell.exe; do
    command -v "$cmd" >/dev/null || die "Missing command: $cmd (enable WSL Windows interop if necessary)."
done
git diff --quiet && git diff --cached --quiet || die 'Commit tracked changes first. Build uses the exact HEAD source.'
head="$(git rev-parse HEAD)"
version="$(python3 -c 'import json; print(json.load(open("src-tauri/tauri.conf.json"))["version"])')"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die 'Expected a stable semantic version.'
python3 - "$version" <<'PY'
import json, re, sys
version = sys.argv[1]
assert json.load(open("package.json"))["version"] == version, "package version mismatch"
assert re.search(r'^version = "([^"]+)"', open("src-tauri/Cargo.toml").read(), re.M).group(1) == version, "Cargo version mismatch"
PY
tag="v$version"
prefix="PT-Analytics-Next_$version"
if ((publish)); then
    if ! command -v gh >/dev/null; then
        ((bootstrap)) || die 'Install GitHub CLI in WSL: sudo apt update && sudo apt install gh; or use --bootstrap.'
        sudo apt-get update
        sudo apt-get install -y gh
    fi
    gh auth status >/dev/null 2>&1 || gh auth login --hostname github.com --web
    remote="$(gh api "repos/$REPO/git/ref/heads/$TARGET_BRANCH" --jq '.object.sha')"
    [[ "$head" == "$remote" ]] || die "HEAD is not the published $TARGET_BRANCH head. git pull --ff-only first (or push your committed changes)."
    if gh release view "$tag" --repo "$REPO" >/dev/null 2>&1; then
        die "Release $tag already exists (possibly a draft); refusing to overwrite. Inspect it or bump the version."
    fi
    existing="$(git ls-remote --tags "https://github.com/$REPO.git" "refs/tags/$tag")"
    [[ -z "$existing" ]] || die "Tag $tag already exists; refusing to publish against an ambiguous source."
fi
# Windows-local staging avoids UNC/WSL path limitations of cmd, npm and MSVC.
win_temp="$(powershell.exe -NoProfile -NonInteractive -Command '[Console]::WriteLine([System.IO.Path]::GetTempPath())' | tr -d '\r')"
temp="$(wslpath -u "$win_temp")"
stage="$(mktemp -d "$temp/pt-analytics-build.XXXXXXXX")"
cleanup() {
    local rc=$?
    if ((rc == 0)); then rm -rf -- "$stage"; else printf 'Build staging retained for debugging: %s\n' "$stage" >&2; fi
}
trap cleanup EXIT
git archive --format=tar "$head" | tar -xf - -C "$stage"
git ls-tree -r -z --name-only "$head" > "$stage/.tracked-manifest"
bash -n "$stage/build-release.sh"
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$(wslpath -w "$stage/scripts/check-powershell.ps1")"
if ((check_only)); then
    printf 'Build script syntax checks passed.\n'
    exit 0
fi
if [[ -d "$ROOT/ui/damage-icons" ]]; then
    mkdir -p "$stage/ui/damage-icons"
    cp -a "$ROOT/ui/damage-icons/." "$stage/ui/damage-icons/"
fi
set --
if ((bootstrap)); then set -- -Bootstrap; fi
printf 'Building %s from %s using Windows native MSVC...\n' "$tag" "$head"
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$(wslpath -w "$stage/scripts/build-windows.ps1")" "$@"
mkdir -p "$ROOT/dist"
portable="$prefix"_x64_portable.exe
installer="$prefix"_x64_setup.exe
source="$prefix"_source.zip
for artifact in "$portable" "$installer"; do
    [[ -s "$stage/dist/$artifact" ]] || die "Missing build output: $artifact"
    cp -f -- "$stage/dist/$artifact" "$ROOT/dist/$artifact"
done
python3 "$stage/scripts/package_source.py" "$stage" "$stage/.tracked-manifest" "$ROOT/dist/$source"
(
    cd "$ROOT/dist"
    sha256sum "$portable" "$installer" "$source" > SHA256SUMS.txt
    sha256sum -c SHA256SUMS.txt
)
mkdir -p "$ROOT/ui/damage-icons"
cp -a "$stage/ui/damage-icons/." "$ROOT/ui/damage-icons/"
printf 'Artifacts: %s/dist\n' "$ROOT"
if ((publish)); then
    remote="$(gh api "repos/$REPO/git/ref/heads/$TARGET_BRANCH" --jq '.object.sha')"
    [[ "$head" == "$remote" ]] || die 'Remote master changed during build; rebuild from the latest commit.'
    existing="$(git ls-remote --tags "https://github.com/$REPO.git" "refs/tags/$tag")"
    [[ -z "$existing" ]] || die "Tag $tag appeared during build; refusing to overwrite."
    gh release create "$tag" \
        "$ROOT/dist/$portable" "$ROOT/dist/$installer" "$ROOT/dist/$source" "$ROOT/dist/SHA256SUMS.txt" \
        --repo "$REPO" --target "$head" --title "PT Analytics Next $tag" \
        --notes-file "$stage/RELEASE_NOTES.md" --draft
    gh release edit "$tag" --repo "$REPO" --draft=false
    gh release view "$tag" --repo "$REPO" --json url --jq '.url'
fi
