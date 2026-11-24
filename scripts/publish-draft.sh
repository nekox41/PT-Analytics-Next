#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Finalize a fully uploaded draft using the REST API (also supported by old gh).
set -Eeuo pipefail
repo="$1"
tag="$2"
commit="$3"
dist="$4"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid release tag' >&2; exit 1; }
[[ "$commit" =~ ^[a-fA-F0-9]{40}$ ]] || { echo 'Invalid release commit' >&2; exit 1; }
id="$(gh api --paginate "repos/$repo/releases?per_page=100" --jq ".[] | select(.tag_name == \"$tag\") | .id")"
[[ "$id" =~ ^[0-9]+$ ]] || { echo "Expected exactly one release for $tag; inspect drafts on GitHub." >&2; exit 1; }
endpoint="repos/$repo/releases/$id"
# Validate exact tag, source commit and all four assets before making it public.
python_cmd=python3
if ! command -v "$python_cmd" >/dev/null; then python_cmd=python; fi
gh api "$endpoint" | "$python_cmd" -c '
import json, pathlib, sys
r=json.load(sys.stdin)
tag,commit,dist=sys.argv[1:]
assert r["tag_name"] == tag and r["target_commitish"] == commit, "Draft tag or source commit mismatch"
assert r["draft"] is True, "Release is not a draft"
prefix="PT-Analytics-Next_"+tag[1:]
names=[prefix+"_x64_portable.exe",prefix+"_x64_setup.exe",prefix+"_source.zip","SHA256SUMS.txt"]
assets={x["name"]:x for x in r["assets"]}
assert set(assets)==set(names), "Draft asset list mismatch"
for name in names:
    local=pathlib.Path(dist)/name
    a=assets[name]
    assert a["state"]=="uploaded" and a["size"]==local.stat().st_size and a["size"]>0, "Incomplete asset: "+name
print("Draft assets verified.")
' "$tag" "$commit" "$dist"
printf 'Publishing draft %s (ID %s)...\n' "$tag" "$id"
if ! gh api --method PATCH "$endpoint" -F draft=false --jq '.html_url'; then
    printf 'Draft retained. Inspect https://github.com/%s/releases or retry: gh api --method PATCH %s -F draft=false\n' "$repo" "$endpoint" >&2
    exit 1
fi
[[ "$(gh api "$endpoint" --jq '.draft')" == false ]] || { echo 'Release is still a draft.' >&2; exit 1; }
printf 'Published: https://github.com/%s/releases/tag/%s\n' "$repo" "$tag"
