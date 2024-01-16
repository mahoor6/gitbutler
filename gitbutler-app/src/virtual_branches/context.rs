use crate::git::diff;
use anyhow::{Context, Result};

pub fn hunk_with_context(
    hunk_diff: &str,
    hunk_start_line: usize,
    is_binary: bool,
    context_lines: usize,
    file_lines_before: &[&str],
) -> Result<diff::Hunk> {
    let diff_lines = hunk_diff
        .lines()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();

    let removed_count = diff_lines
        .iter()
        .filter(|line| line.starts_with('-'))
        .count();
    let added_count = diff_lines
        .iter()
        .filter(|line| line.starts_with('+'))
        .count();

    // Get context lines before the diff
    let mut context_before = Vec::new();
    for i in 1..=context_lines {
        if hunk_start_line > i {
            let idx = hunk_start_line - i - 1;
            if idx < file_lines_before.len() {
                if let Some(l) = file_lines_before.get(idx) {
                    let mut s = (*l).to_string();
                    s.insert(0, ' ');
                    context_before.push(s);
                }
            }
        }
    }
    context_before.reverse();

    // Get context lines after the diff
    let mut context_after = Vec::new();
    let end = context_lines - 1;
    for i in 0..=end {
        let idx = hunk_start_line + removed_count + i - 1;
        if idx < file_lines_before.len() {
            if let Some(l) = file_lines_before.get(idx) {
                let mut s = (*l).to_string();
                s.insert(0, ' ');
                context_after.push(s);
            }
        }
    }

    let header = &diff_lines[0];
    let body = &diff_lines[1..];

    // Update unidiff header values
    let header = header
        .split(|c| c == ' ' || c == '@')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let start_line_before = header[0].split(',').collect::<Vec<_>>()[0]
        .parse::<isize>()
        .context("failed to parse unidiff header value for start line before")?
        .unsigned_abs()
        .saturating_sub(context_before.len());
    let line_count_before = removed_count + context_before.len() + context_after.len();
    let start_line_after = header[1].split(',').collect::<Vec<_>>()[0]
        .parse::<isize>()
        .context("failed to parse unidiff header value for start line after")?
        .unsigned_abs()
        .saturating_sub(context_before.len());
    let line_count_after = added_count + context_before.len() + context_after.len();
    let header = format!(
        "@@ -{},{} +{},{} @@",
        start_line_before, line_count_before, start_line_after, line_count_after
    );

    // Update unidiff body with context lines
    let mut b = Vec::new();
    b.extend(context_before.clone());
    b.extend_from_slice(body);
    b.extend(context_after.clone());
    let body = b;

    // Construct a new diff with updated header and body
    let mut diff_lines = Vec::new();
    diff_lines.push(header);
    diff_lines.extend(body);
    let mut diff = diff_lines.join("\n");
    // Add trailing newline
    diff.push('\n');

    #[allow(clippy::cast_possible_truncation)]
    let hunk = diff::Hunk {
        diff,
        old_start: start_line_before as u32,
        old_lines: line_count_before as u32,
        new_start: start_line_after as u32,
        new_lines: line_count_after as u32,
        binary: is_binary,
    };
    Ok(hunk)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replace_line_mid_file() {
        let hunk_diff = r#"@@ -8 +8 @@ default = ["serde", "rusqlite"]
-serde = ["dep:serde", "uuid/serde"]
+SERDE = ["dep:serde", "uuid/serde"]
"#;
        let with_ctx = hunk_with_context(hunk_diff, 8, false, 3, &file_lines()).unwrap();
        assert_eq!(
            with_ctx.diff,
            r#"@@ -5,7 +5,7 @@
 
 [features]
 default = ["serde", "rusqlite"]
-serde = ["dep:serde", "uuid/serde"]
+SERDE = ["dep:serde", "uuid/serde"]
 rusqlite = ["dep:rusqlite"]
 
 [dependencies]
"#
        );
        assert_eq!(with_ctx.old_start, 5);
        assert_eq!(with_ctx.old_lines, 7);
        assert_eq!(with_ctx.new_start, 5);
        assert_eq!(with_ctx.new_lines, 7);
    }

    #[test]
    fn replace_line_top_file() {
        let hunk_diff = r#"@@ -2 +2 @@
-name = "gitbutler-core"
+NAME = "gitbutler-core"
"#;
        let with_ctx = hunk_with_context(hunk_diff, 2, false, 3, &file_lines()).unwrap();
        assert_eq!(
            with_ctx.diff,
            r#"@@ -1,5 +1,5 @@
 [package]
-name = "gitbutler-core"
+NAME = "gitbutler-core"
 version = "0.0.0"
 edition = "2021"
 
"#
        );
        assert_eq!(with_ctx.old_start, 1);
        assert_eq!(with_ctx.old_lines, 5);
        assert_eq!(with_ctx.new_start, 1);
        assert_eq!(with_ctx.new_lines, 5);
    }

    #[test]
    fn replace_line_start_file() {
        let hunk_diff = "@@ -1 +1 @@
-[package]
+[PACKAGE]
";
        let with_ctx = hunk_with_context(hunk_diff, 1, false, 3, &file_lines()).unwrap();
        assert_eq!(
            with_ctx.diff,
            r#"@@ -1,4 +1,4 @@
-[package]
+[PACKAGE]
 name = "gitbutler-core"
 version = "0.0.0"
 edition = "2021"
"#
        );
        assert_eq!(with_ctx.old_start, 1);
        assert_eq!(with_ctx.old_lines, 4);
        assert_eq!(with_ctx.new_start, 1);
        assert_eq!(with_ctx.new_lines, 4);
    }

    #[test]
    fn replace_line_bottom_file() {
        let hunk_diff = "@@ -13 +13 @@
-serde = { workspace = true, optional = true }
+SERDE = { workspace = true, optional = true }
";
        let with_ctx = hunk_with_context(hunk_diff, 13, false, 3, &file_lines()).unwrap();
        assert_eq!(
            with_ctx.diff,
            r#"@@ -10,5 +10,5 @@
 
 [dependencies]
 rusqlite = { workspace = true, optional = true }
-serde = { workspace = true, optional = true }
+SERDE = { workspace = true, optional = true }
 uuid = { workspace = true, features = ["v4", "fast-rng"] }
"#
        );
        assert_eq!(with_ctx.old_start, 10);
        assert_eq!(with_ctx.old_lines, 5);
        assert_eq!(with_ctx.new_start, 10);
        assert_eq!(with_ctx.new_lines, 5);
    }

    fn file_lines() -> Vec<&'static str> {
        let file_lines_before = r#"[package]
name = "gitbutler-core"
version = "0.0.0"
edition = "2021"

[features]
default = ["serde", "rusqlite"]
serde = ["dep:serde", "uuid/serde"]
rusqlite = ["dep:rusqlite"]

[dependencies]
rusqlite = { workspace = true, optional = true }
serde = { workspace = true, optional = true }
uuid = { workspace = true, features = ["v4", "fast-rng"] }
"#;
        file_lines_before.lines().collect::<Vec<_>>()
    }
}
