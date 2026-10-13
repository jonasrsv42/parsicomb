//! The README's examples print what the README shows
//!
//! Doctests check that every example compiles and runs; this runs each
//! example followed by "prints" and a `text` block, and compares its output
//! with that block.

use std::fs;
use std::path::Path;
use std::process::Command;

/// An example's code and the output the README shows for it
struct Example {
    line: usize,
    text_line: usize,
    code: String,
    expected: String,
}

/// The contents of the fenced block opening at `lines[start]` with `fence`,
/// and the index of its closing line
fn block(lines: &[&str], start: usize, fence: &str) -> Option<(String, usize)> {
    if lines.get(start)?.trim_end() != fence {
        return None;
    }
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .position(|l| l.trim_end() == "```")?;
    Some((lines[start + 1..end].join("\n"), end))
}

/// Every `rust` block followed by a line ending in "prints" (or "prints:")
/// and a `text` block
fn printing_examples(readme: &str) -> Vec<Example> {
    let lines: Vec<&str> = readme.lines().collect();
    let next_nonblank = |from: usize| (from..lines.len()).find(|&i| !lines[i].trim().is_empty());
    let mut examples = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some((code, end)) = block(&lines, i, "```rust") else {
            i += 1;
            continue;
        };
        i = end + 1;
        let Some(intro) = next_nonblank(i) else { break };
        if !lines[intro]
            .trim_end()
            .trim_end_matches(':')
            .ends_with("prints")
        {
            continue;
        }
        let Some(text) = next_nonblank(intro + 1) else {
            break;
        };
        if let Some((expected, text_end)) = block(&lines, text, "```text") {
            examples.push(Example {
                line: end + 1,
                text_line: text + 1,
                code,
                expected,
            });
            i = text_end + 1;
        }
    }
    examples
}

#[test]
fn readme_examples_print_what_the_readme_shows() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(manifest_dir.join("README.md")).unwrap();
    let examples = printing_examples(&readme);
    // Every `text` block is an example's output, so none is left unchecked
    let unchecked: Vec<usize> = readme
        .lines()
        .enumerate()
        .filter(|(_, l)| l.trim_end() == "```text")
        .map(|(i, _)| i + 1)
        .filter(|line| !examples.iter().any(|e| e.text_line == *line))
        .collect();
    assert!(!examples.is_empty());
    assert!(
        unchecked.is_empty(),
        "README lines {unchecked:?}: a `text` block doesn't follow a `rust` block and a \"prints\" line"
    );

    // One scratch crate with a binary per example, built once
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("readme_output");
    let bin_dir = dir.join("src/bin");
    let _ = fs::remove_dir_all(&bin_dir);
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"readme_output\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nparsicomb = {{ path = {:?} }}\n\n[workspace]\n",
            manifest_dir
        ),
    )
    .unwrap();
    for (n, example) in examples.iter().enumerate() {
        let code = if example.code.contains("fn main()") {
            example.code.clone()
        } else {
            format!("fn main() {{\n{}\n}}\n", example.code)
        };
        fs::write(bin_dir.join(format!("example_{n}.rs")), code).unwrap();
    }

    let target = dir.join("target");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--offline", "--bins"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", &target)
        .status()
        .unwrap();
    assert!(status.success(), "building the README examples failed");

    for (n, example) in examples.iter().enumerate() {
        let output = Command::new(target.join("debug").join(format!("example_{n}")))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "README line {} failed:\n{}",
            example.line,
            String::from_utf8_lossy(&output.stderr)
        );
        let printed = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            printed.trim_end(),
            example.expected.trim_end(),
            "README line {}: printed output differs from the README",
            example.line
        );
    }
}
