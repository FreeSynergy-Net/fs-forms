//! The place of the cards in a column: `order` is read as the sequence (`forms.txt`: "read = sequence, write = target").
//!
//! Pattern: none (pure function).

use std::cmp::Ordering;
use std::collections::HashMap;

use fs_components_kanban::Column;

use super::FormRow;

/// The number in the channel `order` of a row (none: the channel is missing, empty or no finite number).
fn order_of(row: &FormRow) -> Option<f64> {
    row.get("order")
        .and_then(|t| t.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
}

/// Sorts the cards of every column STABLY by the `order` of their row; cards without a number go to the end in the order they came.
pub(super) fn sort_by_order(columns: &mut [Column], rows: &[FormRow]) {
    let orders: HashMap<&str, Option<f64>> =
        rows.iter().map(|r| (r.key.as_str(), order_of(r))).collect();
    let place = |key: &str| orders.get(key).copied().flatten();
    for column in columns {
        column
            .cards
            .sort_by(|a, b| match (place(&a.key), place(&b.key)) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            });
    }
}
