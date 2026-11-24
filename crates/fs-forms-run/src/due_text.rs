//! The text of a point in time on a card: a JSCalendar `LocalDateTime` (RFC 8984: `2026-11-26T12:00:00`, no zone; the zone is the
//! separate property `timeZone`) or a `UTCDateTime` (`...Z`) or an offset form is shown as `2026-11-26 12:00`, with `UTC` or the
//! offset behind it when the value carries one. A value that is none of these is shown AS IT IS (nothing is guessed, nothing is hidden).
//!
//! Pattern: Adapter (value text in, display text out; pure). Abweichung: no `fs-jscalendar` dependency, because `tests/deps.rs` keeps the
//! host's dependencies narrow; the reader of the exact wire form is `fs_jscalendar::time::format`, the positions here are the same.

/// The display text of `raw`; `raw` itself when it is not a date-time of the forms above.
#[must_use]
pub fn display(raw: &str) -> String {
    let b = raw.as_bytes();
    let digits =
        |r: std::ops::Range<usize>| b.get(r).is_some_and(|s| s.iter().all(u8::is_ascii_digit));
    let plain = b.len() >= 16
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && matches!(b[10], b'T' | b' ')
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16);
    if !plain {
        return raw.to_owned();
    }
    // after HH:MM: optional `:SS`, optional fraction, then nothing, `Z` or `+hh:mm` / `-hh:mm`
    let mut rest = &raw[16..];
    if rest.as_bytes().first() == Some(&b':') && rest.as_bytes().get(1..3).is_some_and(digits_of) {
        rest = rest.get(3..).unwrap_or("");
        if let Some(f) = rest.strip_prefix('.') {
            rest = f.trim_start_matches(|c: char| c.is_ascii_digit());
        }
    }
    let zone = match rest {
        "" => Some(String::new()),
        "Z" => Some(" UTC".to_owned()),
        o if o.len() == 6
            && matches!(o.as_bytes()[0], b'+' | b'-')
            && digits_of(&o.as_bytes()[1..3])
            && o.as_bytes()[3] == b':'
            && digits_of(&o.as_bytes()[4..6]) =>
        {
            Some(format!(" {o}"))
        }
        _ => None,
    };
    match zone {
        Some(z) => format!("{} {}{z}", &raw[0..10], &raw[11..16]),
        None => raw.to_owned(),
    }
}

fn digits_of(s: &[u8]) -> bool {
    s.iter().all(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::display;

    #[test]
    fn a_local_date_time_loses_the_t_and_the_seconds() {
        assert_eq!(display("2026-11-26T12:00:00"), "2026-11-26 12:00");
    }

    #[test]
    fn a_utc_and_an_offset_value_name_their_zone() {
        assert_eq!(display("2026-11-26T12:00:00Z"), "2026-11-26 12:00 UTC");
        assert_eq!(
            display("2026-11-26T12:00:00+01:00"),
            "2026-11-26 12:00 +01:00"
        );
    }

    #[test]
    fn anything_else_stays_as_it_is() {
        assert_eq!(display("12 Nov"), "12 Nov");
        assert_eq!(
            display("2026-11-26T12:00:00-00-00T00:00:00"),
            "2026-11-26T12:00:00-00-00T00:00:00"
        );
        assert_eq!(display(""), "");
    }
}
