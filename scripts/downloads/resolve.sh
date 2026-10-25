#!/usr/bin/env bash
# The downloads workflow's first job (.github/workflows/downloads.yml): checks
# the tag against the commit the run is on, and finds the tag's release.
#
# It reads the run from the environment: EVENT (the event's name), PR (a pull
# request's number), TAG, COMMIT (the commit the run is on), GH_REPO and GH_TOKEN
# for gh, and GitHub's own GITHUB_REF_TYPE, GITHUB_REF_NAME, GITHUB_OUTPUT and
# GITHUB_STEP_SUMMARY. It writes `name` (the tag, or pr-N on a dry run) and
# `has_draft` (whether a draft release for the tag is already there) to
# GITHUB_OUTPUT, and stops with an error for anything that mustn't be built.
# scripts/downloads/test.sh runs it against a stand-in for gh.
set -euo pipefail
fail() { echo "::error::$1"; exit 1; }

if [ "$EVENT" = pull_request ]; then
    # A dry run: the zips are named after the pull request.
    { echo "name=pr-$PR"; echo "has_draft=false"; } >> "$GITHUB_OUTPUT"
    echo "A dry run for pull request #$PR, from $COMMIT (the pull request merged into its base): no release is touched." >> "$GITHUB_STEP_SUMMARY"
    exit 0
fi

# Only a version tag, which also keeps the tag safe in the URLs, queries and
# file names below.
[[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] \
    || fail "A release's tag looks like v0.2.0, or v0.3.0-beta.1."
if [ "$GITHUB_REF_TYPE" = tag ]; then
    from="the commit of the tag $GITHUB_REF_NAME"
else
    from="the newest commit on $GITHUB_REF_NAME"
fi

# A tag already on GitHub must be on the commit built. An annotated tag
# points at a tag object, which points at the commit.
tag_object=$(gh api "repos/$GH_REPO/git/matching-refs/tags/$TAG" \
    --jq ".[] | select(.ref == \"refs/tags/$TAG\") | \"\(.object.type) \(.object.sha)\"")
tag_on_github=false
if [ -n "$tag_object" ]; then
    tag_on_github=true
    read -r type tag_commit <<<"$tag_object"
    if [ "$type" = tag ]; then
        tag_commit=$(gh api "repos/$GH_REPO/git/tags/$tag_commit" --jq .object.sha)
    fi
    [ "$tag_commit" = "$COMMIT" ] \
        || fail "The tag $TAG is on $tag_commit, but this run is on $COMMIT, $from. Run it from the tag instead (under \"Use workflow from\", or gh workflow run downloads.yml --ref $TAG -f tag=$TAG)."
fi

# The release for the tag: a draft takes the zips, a published one is
# refused. Drafts make no tag, so there can be several.
releases=$(gh api "repos/$GH_REPO/releases?per_page=100" --paginate \
    --jq ".[] | select(.tag_name == \"$TAG\") | \"\(.draft) \(.target_commitish)\"")
has_draft=false
if [ -n "$releases" ]; then
    [ "$(wc -l <<<"$releases")" -eq 1 ] \
        || fail "There's more than one release for $TAG. Delete all but one, then run this again."
    read -r is_draft draft_target <<<"$releases"
    [ "$is_draft" = true ] \
        || fail "The release for $TAG is already published. This only adds downloads to a draft release, so that nothing reaches players before it's been checked."
    has_draft=true
    # Until the tag exists, the draft's own commit (or branch) is where
    # publishing would make it, so it must be the one built.
    if [ "$tag_on_github" = false ]; then
        draft_commit=$(gh api "repos/$GH_REPO/commits/$draft_target" --jq .sha)
        [ "$draft_commit" = "$COMMIT" ] \
            || fail "The draft release for $TAG names $draft_commit, but this run is on $COMMIT, $from. Point the draft at this commit, or push the tag on that one and run this from the tag."
    fi
fi

# The game at that commit names the tag's version, so --version says the
# release. Checked before anything's built. A tag with a hyphen may differ
# after the hyphen, for a test (v0.1.0-test.1 on 0.1.0), but not before it.
version=$(gh api "repos/$GH_REPO/contents/Cargo.toml?ref=$COMMIT" -H "Accept: application/vnd.github.raw+json" \
    | sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p')
if [ "v$version" != "$TAG" ]; then
    mismatch="The game at $COMMIT says it's version $version, not ${TAG#v}."
    if [[ "$TAG" != *-* || "v${version%%-*}" != "${TAG%%-*}" ]]; then
        fail "$mismatch Raise its version first (docs/agents/releases.md, \"Making one\")."
    fi
    echo "::warning::$mismatch That's fine for a test tag."
    echo "$mismatch That's fine for a test tag." >> "$GITHUB_STEP_SUMMARY"
fi

{ echo "name=$TAG"; echo "has_draft=$has_draft"; } >> "$GITHUB_OUTPUT"
{
    echo "Building $TAG from $COMMIT, $from."
    if [ "$has_draft" = true ]; then
        echo "The zips go on its draft release."
    else
        echo "There's no release for $TAG yet, so the zips go on a new draft release."
    fi
} >> "$GITHUB_STEP_SUMMARY"
