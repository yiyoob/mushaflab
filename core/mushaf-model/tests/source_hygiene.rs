//! Rust code in `core/` keeps string and char literals ASCII, so no invisible or look-alike
//! character (U+00A0, U+0640, presentation forms) can hide in a comparison: write `\u{...}`.
//! Comments may contain Arabic (CLAUDE.md conventions). Test fixtures must be ASCII throughout.
//! The editing tools used on this repo have turned written escapes into literal characters
//! before; this test catches that.

use std::fs;
use std::path::{Path, PathBuf};

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                files(&p, ext, out);
            }
        } else if ext.is_empty() || p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

/// Line numbers of non-ASCII characters outside comments.
fn non_ascii_code(src: &str) -> Vec<usize> {
    #[derive(PartialEq)]
    enum S {
        Code,
        LineComment,
        BlockComment(u32),
        Str,
        RawStr(usize),
    }
    let chars: Vec<char> = src.chars().collect();
    let (mut s, mut line, mut i, mut out) = (S::Code, 1, 0, Vec::new());
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if !c.is_ascii() && !matches!(s, S::LineComment | S::BlockComment(_)) {
            out.push(line);
        }
        match s {
            S::Code => match (c, next) {
                ('/', Some('/')) => s = S::LineComment,
                ('/', Some('*')) => (s, i) = (S::BlockComment(1), i + 1),
                ('"', _) => s = S::Str,
                ('r', Some('"' | '#')) => {
                    let hashes = chars[i + 1..].iter().take_while(|&&h| h == '#').count();
                    if chars.get(i + 1 + hashes) == Some(&'"') {
                        (s, i) = (S::RawStr(hashes), i + 1 + hashes);
                    }
                }
                ('\'', Some('\\')) => {
                    // escaped char literal: skip to its closing quote
                    let end = chars[i + 2..].iter().position(|&q| q == '\'').unwrap_or(0);
                    i += 2 + end;
                }
                ('\'', Some(ch)) if chars.get(i + 2) == Some(&'\'') => {
                    if !ch.is_ascii() {
                        out.push(line);
                    }
                    i += 2;
                }
                _ => {}
            },
            S::LineComment if c == '\n' => s = S::Code,
            S::BlockComment(d) => match (c, next) {
                ('*', Some('/')) => {
                    (s, i) = (
                        if d == 1 {
                            S::Code
                        } else {
                            S::BlockComment(d - 1)
                        },
                        i + 1,
                    )
                }
                ('/', Some('*')) => (s, i) = (S::BlockComment(d + 1), i + 1),
                _ => {}
            },
            S::Str => match c {
                '\\' => {
                    if next == Some('\n') {
                        line += 1;
                    }
                    i += 1;
                }
                '"' => s = S::Code,
                _ => {}
            },
            S::RawStr(h) => {
                if c == '"' && chars[i + 1..].iter().take(h).filter(|&&x| x == '#').count() == h {
                    (s, i) = (S::Code, i + h);
                }
            }
            S::LineComment => {}
        }
        if c == '\n' {
            line += 1;
        }
        i += 1;
    }
    out.dedup();
    out
}

#[test]
fn rust_literals_are_ascii() {
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut rs = Vec::new();
    files(core, "rs", &mut rs);
    assert!(
        rs.len() > 10,
        "found only {} .rs files under {}",
        rs.len(),
        core.display()
    );
    let problems: Vec<String> = rs
        .iter()
        .flat_map(|p| {
            let src = fs::read_to_string(p).unwrap();
            non_ascii_code(&src)
                .into_iter()
                .map(move |l| format!("{}:{l}", p.display()))
        })
        .collect();
    assert!(
        problems.is_empty(),
        "non-ASCII outside comments:\n{}",
        problems.join("\n")
    );
}

#[test]
fn fixtures_are_ascii() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut all = Vec::new();
    files(&fixtures, "", &mut all);
    let bad: Vec<String> = all
        .iter()
        .filter(|p| !fs::read(p).unwrap().is_ascii())
        .map(|p| p.display().to_string())
        .collect();
    assert!(bad.is_empty(), "non-ASCII fixture files: {bad:?}");
}

#[test]
fn scanner_flags_literals_but_not_comments() {
    let nbsp = '\u{a0}';
    let src = format!(
        "// ok {nbsp}\n/* ok {nbsp} /* nested */ {nbsp} */\nlet a = \"{nbsp}\";\nlet b = '{nbsp}';\n\
         let c = r#\"{nbsp}\"#;\nlet d = \"//\"; let e = \"{nbsp}\";\nlet f = '\\u{{a0}}'; let g = \"\\\"\"; // {nbsp}\n\
         fn h<'a>(x: &'a str) {{}} let i = \"{nbsp}\";\n"
    );
    assert_eq!(non_ascii_code(&src), vec![3, 4, 5, 6, 8]);
}
