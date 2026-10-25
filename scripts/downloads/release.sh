#!/usr/bin/env bash
# The downloads workflow's last job (.github/workflows/downloads.yml): puts
# the three zips in zips/ on the tag's draft release, making the draft when
# there isn't one. It never publishes: the owner does, after checking it.
#
# It reads TAG, COMMIT (the commit the zips were built from) and HAS_DRAFT
# (from scripts/downloads/resolve.sh) from the environment, with GH_REPO and
# GH_TOKEN for gh, and a new draft's notes from docs/releases/<tag>.md in the
# folder it runs in. scripts/downloads/test.sh runs it against a stand-in for
# gh.
set -euo pipefail

ls -l zips
[ "$(find zips -name '*.zip' | wc -l)" -eq 3 ] || { echo "::error::Expected three zips."; exit 1; }
# A tag with a hyphen, such as v0.3.0-beta.1, is a pre-release, so
# publishing it doesn't make it the newest release the README points at.
prerelease=()
if [[ "$TAG" == *-* ]]; then prerelease=(--prerelease); fi

if [ "$HAS_DRAFT" = true ]; then
    # Still a draft, just before it's touched.
    is_draft=$(gh release view "$TAG" --json isDraft --jq .isDraft)
    if [ "$is_draft" != true ]; then
        echo "::error::The release for $TAG was published while this ran, so its zips weren't added."
        exit 1
    fi
    gh release edit "$TAG" --target "$COMMIT" "${prerelease[@]}"
    gh release upload "$TAG" zips/*.zip --clobber
else
    # The notes' heading is the release's title, and the rest its
    # description.
    file="docs/releases/$TAG.md"
    notes=$(mktemp)
    title=""
    if [ -f "$file" ]; then
        title=$(sed -n '1s/^# *//p' "$file")
        if [ -n "$title" ]; then
            tail -n +2 "$file" | sed '/./,$!d' > "$notes"
        else
            cp "$file" "$notes"
        fi
    else
        echo "Downloads for $TAG, built from $COMMIT. There was no $file at that commit to describe it." > "$notes"
    fi
    gh release create "$TAG" zips/*.zip --draft --target "$COMMIT" \
        --title "${title:-$TAG}" --notes-file "$notes" "${prerelease[@]}"
fi

url=$(gh release view "$TAG" --json url --jq .url)
{
    echo "## Downloads for $TAG"
    echo
    echo "The zips are on the draft release: $url"
    echo
    echo "Check all three are there, open the one for your computer, then publish the release (Edit, then Publish release). Nobody else can see it until then."
} >> "$GITHUB_STEP_SUMMARY"
