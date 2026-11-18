//! Rows of the `forms_*` tables: a typed cell, a row ordered like the table spec, and the checks against the spec (column count,
//! NULL rules, closed word lists). The mapping of a layout to rows is in `to_rows`, the way back in `from_rows`.
//!
//! Pattern: Value Object (cells and rows) with a small builder.

use std::collections::BTreeMap;

use crate::spec::{Logical, Table};
use crate::tables_overlay::ALL;

/// One value of a row. Booleans and times are integers, references and IRIs are text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    /// NULL.
    Null,
    /// Integer, boolean (0 or 1) or UTC milliseconds.
    Int(i64),
    /// Text.
    Text(String),
}

/// A row: one cell per column of the table spec, in the order of the spec.
pub type Row = Vec<Cell>;

/// All rows of one layout (or one write), per table name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    /// Rows per table.
    pub tables: BTreeMap<&'static str, Vec<Row>>,
    /// Mistakes of the mapping itself (an unknown column name); empty when the mapping is right.
    pub problems: Vec<String>,
}

pub(crate) fn text(s: &str) -> Cell {
    Cell::Text(s.to_owned())
}
pub(crate) fn opt_text(s: Option<&String>) -> Cell {
    s.map_or(Cell::Null, |s| text(s))
}
pub(crate) fn int(n: impl Into<i64>) -> Cell {
    Cell::Int(n.into())
}
pub(crate) fn opt_int(n: Option<u32>) -> Cell {
    n.map_or(Cell::Null, int)
}

/// Builds one row by column name.
pub(crate) struct RowBuilder<'t> {
    table: &'t Table,
    cells: Row,
    problems: Vec<String>,
}

impl<'t> RowBuilder<'t> {
    pub(crate) fn new(table: &'t Table) -> Self {
        Self {
            table,
            cells: vec![Cell::Null; table.cols.len()],
            problems: Vec::new(),
        }
    }
    pub(crate) fn set(mut self, name: &str, cell: Cell) -> Self {
        match self.table.cols.iter().position(|c| c.name == name) {
            Some(i) => self.cells[i] = cell,
            None => self
                .problems
                .push(format!("{}: no column {name}", self.table.name)),
        }
        self
    }
    pub(crate) fn push_into(self, rows: &mut Rows) {
        rows.problems.extend(self.problems);
        rows.tables
            .entry(self.table.name)
            .or_default()
            .push(self.cells);
    }
}

/// Reads one row by column name.
pub(crate) struct RowReader<'a> {
    table: &'a Table,
    row: &'a Row,
}

impl<'a> RowReader<'a> {
    pub(crate) fn new(table: &'a Table, row: &'a Row) -> Self {
        Self { table, row }
    }
    fn cell(&self, name: &str) -> &'a Cell {
        self.table
            .cols
            .iter()
            .position(|c| c.name == name)
            .and_then(|i| self.row.get(i))
            .unwrap_or(&Cell::Null)
    }
    pub(crate) fn text(&self, name: &str) -> Option<String> {
        match self.cell(name) {
            Cell::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
    pub(crate) fn int(&self, name: &str) -> Option<i64> {
        match self.cell(name) {
            Cell::Int(n) => Some(*n),
            _ => None,
        }
    }
    pub(crate) fn uint(&self, name: &str) -> Option<u32> {
        self.int(name).and_then(|n| u32::try_from(n).ok())
    }
}

/// Checks rows against the table specs: known table, column count, NULL only where allowed, cell type, closed words.
/// Returns every mistake, empty when the rows fit.
#[must_use]
pub fn validate(rows: &Rows) -> Vec<String> {
    let mut out = rows.problems.clone();
    for (name, list) in &rows.tables {
        let Some(t) = ALL.iter().find(|t| t.name == *name) else {
            out.push(format!("unknown table {name}"));
            continue;
        };
        for (n, row) in list.iter().enumerate() {
            if row.len() != t.cols.len() {
                out.push(format!(
                    "{name}[{n}]: {} cells for {} columns",
                    row.len(),
                    t.cols.len()
                ));
                continue;
            }
            for (col, cell) in t.cols.iter().zip(row) {
                out.extend(check_cell(name, n, col.name, col.null, col.logical, cell));
            }
        }
    }
    out
}

fn check_cell(
    table: &str,
    n: usize,
    col: &str,
    null: bool,
    ty: Logical,
    cell: &Cell,
) -> Option<String> {
    let at = format!("{table}[{n}].{col}");
    match (cell, ty) {
        (Cell::Null, _) => (!null).then(|| format!("{at}: NULL in a NOT NULL column")),
        (Cell::Int(_), Logical::Int | Logical::Utc | Logical::Bool)
        | (Cell::Text(_), Logical::Text | Logical::FreeText | Logical::Iri | Logical::Ref) => None,
        (Cell::Text(s), Logical::Enum(words)) => {
            (!words.contains(&s.as_str())).then(|| format!("{at}: '{s}' is not in the word list"))
        }
        _ => Some(format!("{at}: wrong cell type")),
    }
}
