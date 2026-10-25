#!/usr/bin/env bash
# Runs the downloads workflow's two scripts, resolve.sh and release.sh, through
# their cases against a stand-in for gh, which answers from each case's files
# and records every call, so their rules are checked without GitHub. The
# workflow's dry run on a pull request runs it first. By hand, from the
# terra-sprites folder (it needs bash and jq):
#
#     scripts/downloads/test.sh
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The stand-in for gh. `gh api` answers from the case's folder ($CASE):
# refs.json (the tags), tagobj.json (an annotated tag), releases.json,
# commits.json (each ref's commit) and version (Cargo.toml's, 0.2.0 if
# absent). `gh release view` answers from view.json. Every call goes in
# $CASE/calls, and a --notes-file is kept as $CASE/notes.
mkdir "$work/bin"
cat > "$work/bin/gh" <<'GH'
#!/usr/bin/env bash
echo "gh $*" >> "$CASE/calls"
jq_filter="."
args=()
while [ $# -gt 0 ]; do
    case "$1" in
        --jq) jq_filter=$2; shift 2 ;;
        --notes-file) cp "$2" "$CASE/notes"; args+=("$1" "$2"); shift 2 ;;
        --paginate) shift ;;
        *) args+=("$1"); shift ;;
    esac
done
set -- "${args[@]}"
answer() { jq -r "$jq_filter" <<<"$1"; }
case "$1 ${2:-}" in
    "api repos/"*/git/matching-refs/tags/*) answer "$(cat "$CASE/refs.json" 2>/dev/null || echo '[]')" ;;
    "api repos/"*/git/tags/*) answer "$(cat "$CASE/tagobj.json")" ;;
    "api repos/"*/releases\?*) answer "$(cat "$CASE/releases.json" 2>/dev/null || echo '[]')" ;;
    "api repos/"*/contents/Cargo.toml\?ref=*)
        printf '[workspace]\n\n[workspace.package]\nversion = "%s"\n\n[profile.dev]\nversion = "9.9.9"\n' \
            "$(cat "$CASE/version" 2>/dev/null || echo 0.2.0)" ;;
    "api repos/"*/commits/*)
        sha=$(jq -r --arg ref "${2##*/commits/}" '.[$ref] // empty' "$CASE/commits.json")
        [ -n "$sha" ] || { echo "gh: No commit found for SHA (HTTP 422)" >&2; exit 1; }
        answer "{\"sha\": \"$sha\"}" ;;
    "release view") answer "$(cat "$CASE/view.json")" ;;
    "release create" | "release edit" | "release upload") ;;
    *) echo "the stand-in gh doesn't know: $*" >&2; exit 1 ;;
esac
GH
chmod +x "$work/bin/gh"
export PATH="$work/bin:$PATH" GH_REPO=Keazra/terra-sprites GH_TOKEN=stand-in

built=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
other=cccccccccccccccccccccccccccccccccccccccc
passed=0
failed=0

# A new case's folder, where `main` and each hash name their commits.
new_case() {
    CASE="$work/$1"
    export CASE
    mkdir "$CASE"
    printf '{"main": "%s", "%s": "%s", "%s": "%s"}\n' "$built" "$built" "$built" "$other" "$other" > "$CASE/commits.json"
    : > "$CASE/calls"
}

# Whether the last script run exited with $1 and its output holds each of the
# texts after it. The output is everything it printed, wrote to
# GITHUB_OUTPUT and GITHUB_STEP_SUMMARY, the calls it made to gh, and the
# notes it gave a new release.
expect() {
    local want=$1 text
    shift
    local ok=true
    [ "$code" = "$want" ] || ok=false
    touch "$CASE/notes"
    for text in "$@"; do
        grep -qF -- "$text" "$CASE/output" "$CASE/calls" "$CASE/notes" || ok=false
    done
    # Whatever happens, nothing publishes.
    if grep -qE -- '--draft=false|--draft false' "$CASE/calls"; then ok=false; fi
    if [ "$ok" = true ]; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
        echo "FAILED: $(basename "$CASE") (exit $code, expected $want; looked for: $*)"
        sed 's/^/    /' "$CASE/output" "$CASE/calls"
    fi
}

# resolve.sh on a run of EVENT for TAG, from a branch or tag.
resolve() {
    local event=$1 tag=$2 ref_type=${3:-branch} ref_name=${4:-main}
    : > "$CASE/out"
    : > "$CASE/summary"
    code=0
    EVENT=$event PR=351 TAG=$tag COMMIT=$built GITHUB_REF_TYPE=$ref_type GITHUB_REF_NAME=$ref_name \
        GITHUB_OUTPUT="$CASE/out" GITHUB_STEP_SUMMARY="$CASE/summary" \
        bash "$here/resolve.sh" > "$CASE/printed" 2>&1 || code=$?
    cat "$CASE/printed" "$CASE/out" "$CASE/summary" > "$CASE/output"
}

# release.sh for TAG with ZIPS zips, a draft already there or not, and the
# notes file's text, if any.
release() {
    local tag=$1 has_draft=$2 zips=$3 notes=${4:-}
    mkdir -p "$CASE/run/zips" "$CASE/run/docs/releases"
    for name in $(seq "$zips"); do echo zip > "$CASE/run/zips/terra-sprites-$tag-$name.zip"; done
    [ -z "$notes" ] || printf '%s' "$notes" > "$CASE/run/docs/releases/$tag.md"
    [ -f "$CASE/view.json" ] || echo '{"isDraft": true, "url": "https://github.com/Keazra/terra-sprites/releases/tag/untagged-1"}' > "$CASE/view.json"
    : > "$CASE/summary"
    code=0
    (cd "$CASE/run" && TAG=$tag COMMIT=$built HAS_DRAFT=$has_draft GITHUB_STEP_SUMMARY="$CASE/summary" \
        bash "$here/release.sh") > "$CASE/printed" 2>&1 || code=$?
    cat "$CASE/printed" "$CASE/summary" > "$CASE/output"
}

# resolve.sh

new_case pull-request
resolve pull_request ""
expect 0 "name=pr-351" "has_draft=false"

new_case not-a-tag
resolve workflow_dispatch v0.2
expect 1 "A release's tag looks like v0.2.0"

new_case nothing-there-yet
resolve workflow_dispatch v0.2.0
expect 0 "name=v0.2.0" "has_draft=false" "the newest commit on main"

new_case old-version
echo 0.1.0 > "$CASE/version"
resolve workflow_dispatch v0.2.0
expect 1 "says it's version 0.1.0, not 0.2.0"

new_case test-tag
echo 0.1.0 > "$CASE/version"
resolve workflow_dispatch v0.1.0-test.1
expect 0 "::warning::" "That's fine for a test tag" "name=v0.1.0-test.1"

new_case beta-on-its-version
echo 0.3.0-beta.1 > "$CASE/version"
resolve workflow_dispatch v0.3.0-beta.1
expect 0 "name=v0.3.0-beta.1"

new_case beta-on-the-last-version
echo 0.2.0 > "$CASE/version"
resolve workflow_dispatch v0.3.0-beta.1
expect 1 "says it's version 0.2.0, not 0.3.0-beta.1"

new_case release-tag-on-a-beta
echo 0.2.0-beta.3 > "$CASE/version"
resolve workflow_dispatch v0.2.0
expect 1 "says it's version 0.2.0-beta.3, not 0.2.0"

new_case draft-on-main
echo '[{"tag_name": "v0.2.0", "draft": true, "target_commitish": "main"}, {"tag_name": "v0.2.0-beta.1", "draft": false, "target_commitish": "main"}]' > "$CASE/releases.json"
resolve workflow_dispatch v0.2.0
expect 0 "has_draft=true" "The zips go on its draft release"

new_case draft-on-another-commit
echo "[{\"tag_name\": \"v0.2.0\", \"draft\": true, \"target_commitish\": \"$other\"}]" > "$CASE/releases.json"
resolve workflow_dispatch v0.2.0
expect 1 "The draft release for v0.2.0 names $other"

new_case published
echo '[{"tag_name": "v0.2.0", "draft": false, "target_commitish": "main"}]' > "$CASE/releases.json"
resolve workflow_dispatch v0.2.0
expect 1 "already published"

new_case two-drafts
echo '[{"tag_name": "v0.2.0", "draft": true, "target_commitish": "main"}, {"tag_name": "v0.2.0", "draft": true, "target_commitish": "main"}]' > "$CASE/releases.json"
resolve workflow_dispatch v0.2.0
expect 1 "more than one release for v0.2.0"

new_case annotated-tag-on-the-commit
echo '[{"ref": "refs/tags/v0.2.0", "object": {"type": "tag", "sha": "dddd"}}, {"ref": "refs/tags/v0.2.0-beta.1", "object": {"type": "commit", "sha": "eeee"}}]' > "$CASE/refs.json"
echo "{\"object\": {\"type\": \"commit\", \"sha\": \"$built\"}}" > "$CASE/tagobj.json"
# Once the tag exists, a draft's own commit no longer matters.
echo "[{\"tag_name\": \"v0.2.0\", \"draft\": true, \"target_commitish\": \"$other\"}]" > "$CASE/releases.json"
resolve workflow_dispatch v0.2.0 tag v0.2.0
expect 0 "has_draft=true" "the commit of the tag v0.2.0"

new_case tag-on-another-commit
echo "[{\"ref\": \"refs/tags/v0.2.0\", \"object\": {\"type\": \"commit\", \"sha\": \"$other\"}}]" > "$CASE/refs.json"
resolve workflow_dispatch v0.2.0
expect 1 "The tag v0.2.0 is on $other" "--ref v0.2.0"

# release.sh

new_case new-draft
release v0.2.0 false 3 $'# v0.2.0: Generations\n\nWhat this milestone is about.\n'
expect 0 "--draft --target $built" "--title v0.2.0: Generations" "untagged-1"
if ! grep -qx "What this milestone is about." "$CASE/notes" || grep -q "^#" "$CASE/notes"; then
    failed=$((failed + 1))
    echo "FAILED: new-draft's description should be the notes without their heading"
fi

new_case new-beta-draft
release v0.3.0-beta.1 false 3
expect 0 "--title v0.3.0-beta.1" "--prerelease" "There was no docs/releases/v0.3.0-beta.1.md"

new_case existing-beta-draft
release v0.3.0-beta.1 true 3
expect 0 "gh release edit v0.3.0-beta.1 --target $built --prerelease" "--clobber"

new_case published-while-it-ran
echo '{"isDraft": false, "url": "x"}' > "$CASE/view.json"
release v0.2.0 true 3
expect 1 "was published while this ran"
if grep -q "release upload" "$CASE/calls"; then failed=$((failed + 1)); echo "FAILED: published-while-it-ran uploaded anyway"; fi

new_case two-zips
release v0.2.0 false 2
expect 1 "Expected three zips."

echo "$passed passed, $failed failed"
[ "$failed" = 0 ]
