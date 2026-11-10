mod parse;

use std::path::Path;

use crate::Error;
use crate::run::git;
use crate::status::Change;

const LIMIT: usize = 5000;
const FULL_CONTEXT: &str = "-U100000";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Hunks,
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Meta(String),
    Hunk(String),
    Context(u32, u32, String),
    Add(u32, String),
    Del(u32, String),
}

#[derive(Clone, Debug)]
pub struct Diff {
    pub rows: Vec<Row>,
    pub hunks: Vec<usize>,
    pub adds: usize,
    pub dels: usize,
    pub width: usize,
    pub truncated: bool,
}

impl Diff {
    pub fn new(rows: Vec<Row>, truncated: bool) -> Self {
        let hunks = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches!(row, Row::Hunk(_)))
            .map(|(index, _)| index)
            .collect();
        let adds = rows
            .iter()
            .filter(|row| matches!(row, Row::Add(..)))
            .count();
        let dels = rows
            .iter()
            .filter(|row| matches!(row, Row::Del(..)))
            .count();
        let highest = rows
            .iter()
            .map(|row| match row {
                Row::Context(old, new, _) => *old.max(new),
                Row::Add(number, _) | Row::Del(number, _) => *number,
                Row::Meta(_) | Row::Hunk(_) => 0,
            })
            .max()
            .unwrap_or(0);
        let width = highest.max(1).to_string().len();

        Self {
            rows,
            hunks,
            adds,
            dels,
            width,
            truncated,
        }
    }

    fn full(rows: Vec<Row>, truncated: bool) -> Self {
        let rows: Vec<Row> = rows
            .into_iter()
            .filter(|row| !matches!(row, Row::Hunk(_)))
            .collect();
        let mut diff = Self::new(rows, truncated);
        diff.hunks = (0..diff.rows.len())
            .filter(|&index| {
                let changed = |at: usize| matches!(diff.rows[at], Row::Add(..) | Row::Del(..));

                changed(index) && (index == 0 || !changed(index - 1))
            })
            .collect();

        diff
    }

    pub fn added(lines: Vec<String>, truncated: bool) -> Self {
        let adds = lines
            .into_iter()
            .zip(1..)
            .map(|(line, number)| Row::Add(number, line));
        let rows = std::iter::once(Row::Meta("New file (untracked)".to_owned()))
            .chain(adds)
            .collect();

        Self::new(rows, truncated)
    }
}

pub fn diff(root: &Path, change: &Change, context: Context) -> Result<Diff, Error> {
    let paths: Vec<&str> = change
        .orig
        .iter()
        .map(String::as_str)
        .chain([change.path.as_str()])
        .collect();
    let text = run(root, &["HEAD"], &paths, context).or_else(|_| {
        let staged = run(root, &["--cached"], &paths, context)?;
        let unstaged = run(root, &[], &paths, context)?;

        Ok::<_, Error>(staged + &unstaged)
    })?;
    let parsed = parse::parse(&text);

    Ok(match context {
        Context::Hunks => parsed,
        Context::Full => Diff::full(parsed.rows, parsed.truncated),
    })
}

fn run(root: &Path, scope: &[&str], paths: &[&str], context: Context) -> Result<String, Error> {
    let lines = match context {
        Context::Hunks => "-U3",
        Context::Full => FULL_CONTEXT,
    };
    let mut args = vec!["diff"];
    args.extend(scope);
    args.extend(["--no-ext-diff", "--no-textconv", lines, "-M", "--"]);
    args.extend(paths);
    let output = git(root, &args)?;

    Ok(String::from_utf8_lossy(&output).into_owned())
}
