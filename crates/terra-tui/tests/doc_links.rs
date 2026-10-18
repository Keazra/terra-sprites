//! Every relative link in the repo's Markdown leads somewhere (design §7.4,
//! A9): to a file or folder git tracks, and, when it names an `#anchor` on a
//! Markdown page, to a heading there. So a file renamed or moved fails this
//! test until the links to it follow. A link into the private docs folder
//! fails too, since every public copy has it empty (AGENTS.md, "Public and
//! private documents").
//!
//! Web links aren't followed, so the test needs no network. The private
//! repo's own pages aren't checked either: git lists its folder as one entry.
//! It lives here rather than in `terra-sim`, whose rules ban `std::fs`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// The private repo's folder, empty in every public copy.
const PRIVATE_DOCS: &str = "docs/terra-sprites-docs";

#[test]
fn every_relative_link_leads_somewhere() {
    let mut repo = Repo::new();
    let pages: Vec<String> = repo
        .files
        .iter()
        .filter(|file| file.ends_with(".md"))
        .cloned()
        .collect();
    let mut broken = Vec::new();
    for page in pages {
        for problem in repo.broken_links(&page, &read(&page)) {
            broken.push(format!("{page}:{problem}"));
        }
    }
    assert!(
        broken.is_empty(),
        "links that lead nowhere:\n{}",
        broken.join("\n")
    );
}

/// The check fails on each kind of broken link, in links, images and
/// reference definitions, and only on those: not on web links, footnotes or
/// brackets in code, which Markdown shows as they are. The good links lean on
/// `README.md` and the manual's "Controls" heading.
#[test]
fn each_kind_of_broken_link_is_named_with_its_line() {
    let page = "\
# An example
## Twice ##
## Twice
[Controls](manual.md#controls), [the README](/README.md), [up](#an-example), [down](#twice-1).
[A heading that isn't there](manual.md#no-such-heading)
[A page that isn't there](reference/no-such-page.md) and ![a picture](none.png).
[A private note](terra-sprites-docs/notes.md)
[Out of the repo](../../outside.md) and [GitHub](https://github.com/x/y.md).
`[In code](nowhere.md)` and [a heading this page lacks](#controls).
[a reference]: nowhere.md \"Its title\"
[^a footnote]: Not a link.
```
[In a fenced block](nowhere.md)
```
~~~
[In another](nowhere.md)
~~~
";
    let broken = Repo::new().broken_links("docs/example.md", page);
    assert_eq!(
        broken,
        [
            "5: `manual.md#no-such-heading`: no such heading there",
            "6: `reference/no-such-page.md`: no such file or folder",
            "6: `none.png`: no such file or folder",
            "7: `terra-sprites-docs/notes.md`: \
             into the private docs folder, which public copies have empty",
            "8: `../../outside.md`: leaves the repo",
            "9: `#controls`: no such heading there",
            "10: `nowhere.md`: no such file or folder",
        ]
    );
}

/// The files git tracks, and the anchors of each page a link has named one
/// on.
struct Repo {
    files: BTreeSet<String>,
    anchors: BTreeMap<String, BTreeSet<String>>,
}

impl Repo {
    fn new() -> Repo {
        Repo {
            files: tracked(),
            anchors: BTreeMap::new(),
        }
    }

    /// The links on `page`, whose text is `text`, that lead nowhere: each
    /// with its line and what's wrong.
    fn broken_links(&mut self, page: &str, text: &str) -> Vec<String> {
        let own_anchors = anchors(text);
        links(text)
            .into_iter()
            .filter_map(|(line, link)| {
                let problem = self.problem(page, &own_anchors, &link)?;
                Some(format!("{line}: `{link}`: {problem}"))
            })
            .collect()
    }

    /// What's wrong with `link` on `page`, whose own headings have
    /// `own_anchors`, if anything.
    fn problem(
        &mut self,
        page: &str,
        own_anchors: &BTreeSet<String>,
        link: &str,
    ) -> Option<&'static str> {
        const NO_HEADING: &str = "no such heading there";
        let (path, anchor) = link.split_once('#').unwrap_or((link, ""));
        if path.is_empty() {
            return (!anchor.is_empty() && !own_anchors.contains(anchor)).then_some(NO_HEADING);
        }
        let Some(target) = resolve(page, path) else {
            return Some("leaves the repo");
        };
        if target == PRIVATE_DOCS || target.starts_with(&format!("{PRIVATE_DOCS}/")) {
            return Some("into the private docs folder, which public copies have empty");
        }
        if !self.files.contains(&target) {
            let folder = format!("{target}/");
            let in_folder = target.is_empty()
                || self
                    .files
                    .range(folder.clone()..)
                    .next()
                    .is_some_and(|file| file.starts_with(&folder));
            return (!in_folder).then_some("no such file or folder");
        }
        if anchor.is_empty() || !target.ends_with(".md") {
            return None;
        }
        let headings = self
            .anchors
            .entry(target)
            .or_insert_with_key(|target| anchors(&read(target)));
        (!headings.contains(anchor)).then_some(NO_HEADING)
    }
}

/// The repo's root folder.
fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// A tracked file's text.
fn read(file: &str) -> String {
    std::fs::read_to_string(root().join(file))
        .unwrap_or_else(|error| panic!("can't read {file}: {error}"))
}

/// Every file git tracks, by its path from the repo's root, but for any
/// deleted from the folder and not yet from git.
fn tracked() -> BTreeSet<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root())
        .output()
        .expect("git runs: this test needs git, in a clone of the repo");
    assert!(
        listing.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&listing.stderr)
    );
    String::from_utf8(listing.stdout)
        .expect("file names are UTF-8")
        .split('\0')
        .filter(|file| !file.is_empty() && root().join(file).exists())
        .map(String::from)
        .collect()
}

/// `path` followed from `page`'s folder, or from the root when it starts
/// with `/`, as GitHub reads it; `None` if it climbs out of the repo.
fn resolve(page: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    if !path.starts_with('/') {
        parts.extend(page.split('/'));
        parts.pop();
    }
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// The targets of a page's relative links and images, `[text](target)`,
/// and of its reference definitions, `[label]: target`, with the number of
/// the line each is on. Links in code don't count, since a fenced block or
/// a `` `code` `` span shows its brackets as they are, and neither do web
/// links, `https:` or `mailto:`. HTML's `href` and `src` aren't read.
fn links(page: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (line, text) in prose(page) {
        // A footnote, `[^1]: …`, is text, not a link.
        if let Some(label) = text.trim_start().strip_prefix('[')
            && !label.starts_with('^')
            && let Some((_, rest)) = label.split_once("]:")
            && let Some(target) = rest.split_whitespace().next()
            && !is_web(target)
        {
            found.push((line, target.to_string()));
        }
        for outside_code in text.split('`').step_by(2) {
            for (at, _) in outside_code.match_indices("](") {
                let target: String = outside_code[at + 2..]
                    .chars()
                    .take_while(|&c| !c.is_whitespace() && c != ')')
                    .collect();
                if !target.is_empty() && !is_web(&target) {
                    found.push((line, target));
                }
            }
        }
    }
    found
}

/// Whether a link starts with a scheme, such as `https:`.
fn is_web(link: &str) -> bool {
    link.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
}

/// A page's lines outside fenced code blocks, numbered from 1.
fn prose(page: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut fence: Option<&str> = None;
    page.lines().enumerate().filter_map(move |(index, line)| {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"]
            .into_iter()
            .find(|marker| trimmed.starts_with(marker));
        match (fence, marker) {
            (None, Some(marker)) => fence = Some(marker),
            (Some(open), Some(marker)) if open == marker => fence = None,
            (None, None) => return Some((index + 1, line)),
            (Some(_), _) => {}
        }
        None
    })
}

/// The anchors GitHub gives a page's headings: each heading's words in lower
/// case, joined by hyphens, with punctuation dropped and `-1`, `-2` added to
/// repeats.
fn anchors(page: &str) -> BTreeSet<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    prose(page)
        .filter_map(|(_, line)| {
            let text = line.trim_start_matches('#');
            let level = line.len() - text.len();
            ((1..=6).contains(&level) && text.starts_with(' ')).then_some(text)
        })
        .map(|heading| {
            let slug: String = heading
                .trim()
                .trim_end_matches('#')
                .trim()
                .to_lowercase()
                .chars()
                .filter(|&c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
                .map(|c| if c == ' ' { '-' } else { c })
                .collect();
            let repeats = seen.entry(slug.clone()).or_default();
            let anchor = match *repeats {
                0 => slug,
                n => format!("{slug}-{n}"),
            };
            *repeats += 1;
            anchor
        })
        .collect()
}
