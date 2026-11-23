//! TSV as in `docs/formats.md`: `#` comment lines before the header, one header row, one row
//! per line, tab-separated cells, no quoting, an empty cell means absent.

use crate::Diag;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// 1-based line number in the file.
    pub line: usize,
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub path: String,
    pub rows: Vec<Row>,
}

/// Parse a table whose header must equal `header` exactly. Every row must have as many cells.
pub fn parse(path: &str, text: &str, header: &[&str]) -> Result<Table, Vec<Diag>> {
    let mut lines = text.split('\n').enumerate().peekable();
    while lines.peek().is_some_and(|(_, l)| l.starts_with('#')) {
        lines.next();
    }
    let Some((hn, h)) = lines.next() else {
        return Err(vec![Diag::new(path, None, "empty file: no header")]);
    };
    let got: Vec<&str> = h.split('\t').collect();
    if got != header {
        return Err(vec![Diag::new(
            path,
            Some(hn + 1),
            format!("header {got:?}, expected {header:?}"),
        )]);
    }
    let mut rows = Vec::new();
    let mut errs = Vec::new();
    for (n, l) in lines {
        if l.is_empty() {
            continue;
        }
        if l.ends_with('\r') {
            errs.push(Diag::new(path, Some(n + 1), "CR line end (use LF)"));
            continue;
        }
        let cells: Vec<String> = l.split('\t').map(String::from).collect();
        if cells.len() != header.len() {
            errs.push(Diag::new(
                path,
                Some(n + 1),
                format!("{} columns, expected {}", cells.len(), header.len()),
            ));
            continue;
        }
        rows.push(Row { line: n + 1, cells });
    }
    if !text.is_empty() && !text.ends_with('\n') {
        errs.push(Diag::new(path, None, "no final newline"));
    }
    if errs.is_empty() {
        Ok(Table {
            path: path.to_string(),
            rows,
        })
    } else {
        Err(errs)
    }
}
