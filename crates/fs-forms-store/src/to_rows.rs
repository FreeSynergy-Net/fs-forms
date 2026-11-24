//! A checked layout to rows of the `forms_*` tables. Element tables carry the stable `elem`; sequence numbers are positions.
//! Overlays, conflicts, locks and the log are written by the store from their own events, not from a layout.
//!
//! Pattern: Adapter (model to rows).

use fs_forms_layout::{
    ActionTarget, Coded, Item, Layout, LayoutCode, LayoutId, Origin, PackageRef, Reference, Rev,
    Utc, Words,
};

use crate::rows::{Cell, RowBuilder, Rows, int, opt_text, text};
use crate::tables as t;
use crate::to_rows_parts::{menu_rows, pages, views};

/// The columns of `forms_layout` that do not come from the description.
#[derive(Clone, Debug)]
pub struct LayoutMeta {
    /// Id.
    pub id: LayoutId,
    /// Origin.
    pub origin: Origin,
    /// Package (package originals).
    pub package: Option<PackageRef>,
    /// Owner (person or import).
    pub owner: Option<Reference>,
    /// Counter.
    pub rev: Rev,
    /// Creation time (set by the service).
    pub created: Utc,
    /// Last change (set by the service).
    pub updated: Utc,
    /// Who changed it last.
    pub updated_by: Reference,
}

pub(crate) fn elem<T>(i: &Item<T>) -> Cell {
    text(i.elem.as_ref().map_or("", |e| e.as_str()))
}

/// Splits `name@>=a.b.c, <d.e.f` into its capability and six integers (major, minor, patch of both bounds), read from the parsed
/// bounds of the layout check: the table holds all three numbers, nothing is cut off.
pub(crate) fn range_parts(range: &str) -> Result<(String, [i64; 6]), Coded<LayoutCode>> {
    let refuse = || {
        Coded::new(LayoutCode::ExportFailed)
            .with("cause", "range-invalid")
            .with("range", range)
    };
    let b = fs_forms_layout::range_bounds(range).ok_or_else(refuse)?;
    let n = |x: u64| i64::try_from(x).map_err(|_| refuse());
    Ok((
        b.name.to_owned(),
        [
            n(b.low.0)?,
            n(b.low.1)?,
            n(b.low.2)?,
            n(b.below.0)?,
            n(b.below.1)?,
            n(b.below.2)?,
        ],
    ))
}

/// The text range of the six integers of a `requires` or `source` row, e.g. `tasks@>=0.2.0, <0.3.0`.
pub(crate) fn range_text(r: &crate::rows::RowReader<'_>) -> String {
    let n = |c: &str| r.int(c).unwrap_or(0);
    format!(
        "{}@>={}.{}.{}, <{}.{}.{}",
        r.text("capability").unwrap_or_default(),
        n("min_major"),
        n("min_minor"),
        n("min_patch"),
        n("below_major"),
        n("below_minor"),
        n("below_patch")
    )
}

fn origin_word(o: Origin) -> &'static str {
    match o {
        Origin::Package => "package",
        Origin::Person => "person",
        Origin::Import => "import",
    }
}

/// Rows of one layout, all tables of the description (not the overlay tables).
///
/// # Errors
/// `export-failed` for a part this cut cannot map yet (filters of views).
pub fn to_rows(layout: &Layout, meta: &LayoutMeta) -> Result<Rows, Coded<LayoutCode>> {
    let d = layout.doc();
    let mut rows = Rows::default();
    let id = text(&meta.id.0.0);
    let pkg = meta.package.as_ref();
    RowBuilder::new(&t::LAYOUT)
        .set("id", id.clone())
        .set("extends", text(&d.key.extends))
        .set("frontend", text(&d.key.frontend))
        .set("frontend_key", text(&d.key.frontend))
        .set("exec", text(&d.exec))
        .set("title_key", text(&d.title_key))
        .set("style", opt_text(d.style.as_ref()))
        .set("format", int(d.format))
        .set("origin", text(origin_word(meta.origin)))
        .set("package", pkg.map_or(Cell::Null, |p| text(&p.package)))
        .set(
            "package_version",
            pkg.map_or(Cell::Null, |p| text(&p.version)),
        )
        .set(
            "owner",
            meta.owner.as_ref().map_or(Cell::Null, |o| text(&o.0)),
        )
        .set(
            "rev",
            Cell::Int(i64::try_from(meta.rev.0).unwrap_or(i64::MAX)),
        )
        .set("created", Cell::Int(meta.created.0))
        .set("updated", Cell::Int(meta.updated.0))
        .set("updated_by", text(&meta.updated_by.0))
        .push_into(&mut rows);
    for (n, r) in d.requires.iter().enumerate() {
        let (cap, v) = range_parts(&r.v.range)?;
        RowBuilder::new(&t::REQUIRES)
            .set("layout", id.clone())
            .set("seq", int(u32::try_from(n).unwrap_or(0)))
            .set("elem", elem(r))
            .set("capability", text(&cap))
            .set("min_major", Cell::Int(v[0]))
            .set("min_minor", Cell::Int(v[1]))
            .set("min_patch", Cell::Int(v[2]))
            .set("below_major", Cell::Int(v[3]))
            .set("below_minor", Cell::Int(v[4]))
            .set("below_patch", Cell::Int(v[5]))
            .push_into(&mut rows);
    }
    for (n, s) in d.sources.iter().enumerate() {
        let (cap, v) = range_parts(&s.v.range)?;
        RowBuilder::new(&t::SOURCE)
            .set("layout", id.clone())
            .set("source", int(u32::try_from(n + 1).unwrap_or(0)))
            .set("elem", elem(s))
            .set("capability", text(&cap))
            .set("min_major", Cell::Int(v[0]))
            .set("min_minor", Cell::Int(v[1]))
            .set("min_patch", Cell::Int(v[2]))
            .set("below_major", Cell::Int(v[3]))
            .set("below_minor", Cell::Int(v[4]))
            .set("below_patch", Cell::Int(v[5]))
            .set("data_type", text(&s.v.data_type))
            .set("scope", s.v.scope.map_or(Cell::Null, |w| text(w.word())))
            .push_into(&mut rows);
    }
    let mut node = 0_u32;
    menu_rows(&d.menu, None, &id, &mut node, &mut rows);
    pages(layout, &id, &mut rows)?;
    for (n, a) in d.actions.iter().enumerate() {
        let (target, command, source, workflow) = match &a.v.target {
            ActionTarget::Command { source, command } => {
                ("command", text(command), int(*source), Cell::Null)
            }
            ActionTarget::Workflow(w) => ("workflow", Cell::Null, Cell::Null, text(&w.0)),
        };
        RowBuilder::new(&t::ACTION)
            .set("layout", id.clone())
            .set("action", int(u32::try_from(n + 1).unwrap_or(0)))
            .set("elem", elem(a))
            .set("trigger", text(a.v.trigger.word()))
            .set("target", text(target))
            .set("command", command)
            .set("source", source)
            .set("workflow", workflow)
            .set(
                "confirm",
                a.v.confirm.map_or(Cell::Null, |c| text(c.word())),
            )
            .push_into(&mut rows);
    }
    views(layout, &id, &mut rows)?;
    for m in &d.messages {
        RowBuilder::new(&t::MESSAGE)
            .set("layout", id.clone())
            .set("key", text(&m.v.key))
            .set("lang", text(&m.v.lang))
            .set("elem", elem(m))
            .set("message", text(&m.v.message))
            .push_into(&mut rows);
    }
    Ok(rows)
}

#[cfg(test)]
mod range_tests {
    use super::range_parts;

    #[test]
    fn a_patch_is_kept_in_its_own_number() {
        assert_eq!(
            range_parts("tasks@>=0.2, <0.3").unwrap(),
            ("tasks".to_owned(), [0, 2, 0, 0, 3, 0])
        );
        assert_eq!(
            range_parts("tasks@>=0.0.1, <0.1.0").unwrap().1,
            [0, 0, 1, 0, 1, 0]
        );
        assert_eq!(
            range_parts("tasks@>=0.2, <0.3.4").unwrap().1,
            [0, 2, 0, 0, 3, 4]
        );
    }

    #[test]
    fn a_range_the_check_would_refuse_is_never_exported_as_zeros() {
        // semver accepts blanks after an operator; the one spelling is `>=1.2, <2.0`. The old hand reader gave [0, 2, 0, 0] for the first.
        for text in [
            "tasks@>= 1.2, < 2.0",
            "tasks@>=0.2.0-rc.1, <0.3",
            "tasks@>=0.3, <0.2",
        ] {
            let e = range_parts(text).unwrap_err();
            assert_eq!(e.param("cause"), Some("range-invalid"), "{text}");
        }
        assert_eq!(
            range_parts("tasks@>=1.2, <2.0").unwrap().1,
            [1, 2, 0, 2, 0, 0]
        );
    }
}
