//! Reads the data-format reference's working examples (design §7.4, A9).
//!
//! Both crates' `tests/reference_docs.rs` include this file with `#[path]`:
//! each test file is its own crate, and terra-tui's can't reach terra-sim's
//! `tests/common` as a module.

/// The example `name` from a reference page: the fenced block after its
/// `<!-- example: name -->` marker, without the fences.
pub fn example(page: &str, name: &str) -> String {
    let marker = format!("<!-- example: {name} -->");
    let after = page
        .split_once(&marker)
        .unwrap_or_else(|| panic!("no example `{name}` in the page"))
        .1;
    let block = after
        .trim_start()
        .strip_prefix("```ron\n")
        .unwrap_or_else(|| panic!("example `{name}` isn't a ```ron block"));
    let end = block
        .find("\n```")
        .unwrap_or_else(|| panic!("example `{name}` isn't closed"));
    block[..end].to_string()
}
