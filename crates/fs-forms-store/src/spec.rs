//! Stand-ins for the table description of fs-objects v5 (`spec::{Col, Logical, Table}`, `migrate::Step`), which has not landed.
//! Same shapes as fs-objects v5 `spec` (field `logical`; the renderer does NOT quote column names, so no column may be an SQL keyword), so the tables in `tables.rs` move over by changing one `use`. Also renders the DDL, so the
//! tables can be checked against a real SQL engine before fs-objects v5 exists.
//!
//! Pattern: Value Object (const table specs) with a renderer.

/// The logical type of a column; the physical type follows from it.
#[derive(Clone, Copy, Debug)]
pub enum Logical {
    /// Boolean (0 or 1).
    Bool,
    /// One of a closed list of words (`CHECK IN`).
    Enum(&'static [&'static str]),
    /// Text of any length (MF2 sources, values).
    FreeText,
    /// Integer.
    Int,
    /// An IRI.
    Iri,
    /// A reference to another object (UUIDv7 text).
    Ref,
    /// Short text.
    Text,
    /// UTC time in milliseconds.
    Utc,
}

/// One column.
#[derive(Clone, Copy, Debug)]
pub struct Col {
    /// Column name.
    pub name: &'static str,
    /// Logical type.
    pub logical: Logical,
    /// May be NULL.
    pub null: bool,
}

impl Col {
    /// A column that must have a value.
    #[must_use]
    pub const fn req(name: &'static str, ty: Logical) -> Self {
        Self {
            name,
            logical: ty,
            null: false,
        }
    }
    /// A column that may be NULL.
    #[must_use]
    pub const fn opt(name: &'static str, ty: Logical) -> Self {
        Self {
            name,
            logical: ty,
            null: true,
        }
    }
}

/// One table.
#[derive(Clone, Copy, Debug)]
pub struct Table {
    /// Table name.
    pub name: &'static str,
    /// Columns in order.
    pub cols: &'static [Col],
    /// Table constraints as SQL text.
    pub constraints: &'static [&'static str],
    /// Index statements.
    pub indexes: &'static [&'static str],
}

/// One migration step; the schema version is the number of steps.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    /// Creates tables (and their indexes).
    Create(&'static [Table]),
    /// Runs one statement.
    Sql(&'static str),
}

impl Col {
    fn ddl(&self) -> String {
        let (sql, check) = match self.logical {
            Logical::Bool => ("INTEGER", Some(format!("{} IN (0,1)", self.name))),
            Logical::Enum(words) => {
                let list: Vec<String> = words.iter().map(|w| format!("'{w}'")).collect();
                (
                    "TEXT",
                    Some(format!("{} IN ({})", self.name, list.join(","))),
                )
            }
            Logical::Int | Logical::Utc => ("INTEGER", None),
            Logical::FreeText | Logical::Iri | Logical::Ref | Logical::Text => ("TEXT", None),
        };
        let null = if self.null { "" } else { " NOT NULL" };
        let check = check.map(|c| format!(" CHECK ({c})")).unwrap_or_default();
        format!("{} {sql}{null}{check}", self.name)
    }
}

impl Table {
    /// The `CREATE TABLE` statement (strict).
    #[must_use]
    pub fn ddl(&self) -> String {
        let mut parts: Vec<String> = self.cols.iter().map(Col::ddl).collect();
        parts.extend(self.constraints.iter().map(|c| (*c).to_owned()));
        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n    {}\n) STRICT",
            self.name,
            parts.join(",\n    ")
        )
    }
}

impl Step {
    /// All statements of the step, in order.
    #[must_use]
    pub fn statements(&self) -> Vec<String> {
        match self {
            Step::Create(tables) => tables
                .iter()
                .flat_map(|t| {
                    std::iter::once(t.ddl()).chain(t.indexes.iter().map(|i| (*i).to_owned()))
                })
                .collect(),
            Step::Sql(s) => vec![(*s).to_owned()],
        }
    }
}
