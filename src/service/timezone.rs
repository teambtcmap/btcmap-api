use crate::Error;
use crate::Result;
use serde::Deserialize;
use serde::Deserializer;
use std::sync::LazyLock;
use time::format_description::well_known::Rfc3339;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, Offset, OffsetResult, TimeZone};
use tzf_rs::EmbeddedFinder;

/// A timestamp supplied by a client. It either carries an explicit UTC offset
/// (including `Z`, which means UTC) or is a floating local wall-clock time that
/// needs a timezone to be placed on the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventTime {
    Absolute(OffsetDateTime),
    Local(PrimitiveDateTime),
}

/// Tri-state update field: `None` leaves it untouched, `Some(None)` clears it,
/// `Some(Some(value))` sets it.
pub type FieldUpdate<T> = Option<Option<T>>;

const LOCAL_FORMAT: &[FormatItem<'_>] = format_description!(
    "[year]-[month]-[day][first [ ][T]][hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
);

impl<'de> Deserialize<'de> for EventTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        if let Ok(datetime) = OffsetDateTime::parse(&raw, &Rfc3339) {
            return Ok(EventTime::Absolute(datetime));
        }
        if let Ok(local) = PrimitiveDateTime::parse(&raw, &LOCAL_FORMAT) {
            return Ok(EventTime::Local(local));
        }
        Err(serde::de::Error::custom(
            "timestamp must be RFC 3339 with an offset (e.g. 2026-08-20T19:00:00+07:00) \
             or a local date-time without an offset (e.g. 2026-08-20T19:00:00) plus a timezone",
        ))
    }
}

/// Loads the ~4 MB timezone boundary dataset once and reuses it.
static FINDER: LazyLock<EmbeddedFinder> = LazyLock::new(EmbeddedFinder::new);

fn infer_zone(lat: f64, lon: f64) -> Result<String> {
    let name = FINDER.get_tz_name(lon, lat);
    if name.is_empty() {
        return Err(Error::Other(format!(
            "Could not infer a timezone for ({lat}, {lon}); pass an explicit timezone instead of \"auto\""
        )));
    }
    Ok(name.to_string())
}

/// Resolves the `timezone` request value: either `"auto"` (infer from the
/// event's coordinates) or an IANA zone name such as `Europe/Berlin`.
fn resolve_zone_name(spec: &str, lat: f64, lon: f64) -> Result<String> {
    if spec.eq_ignore_ascii_case("auto") {
        return infer_zone(lat, lon);
    }
    if timezones::get_by_name(spec).is_none() {
        return Err(Error::Other(format!("Unknown timezone: {spec}")));
    }
    Ok(spec.to_string())
}

/// Converts a floating local wall-clock time in `zone_name` into an absolute
/// instant, applying the zone's DST rules for that date.
fn local_to_utc(zone_name: &str, local: PrimitiveDateTime) -> Result<OffsetDateTime> {
    let zone = timezones::get_by_name(zone_name)
        .ok_or_else(|| Error::Other(format!("Unknown timezone: {zone_name}")))?;
    // `get_offset_local` expects the wall-clock fields encoded as if they were
    // UTC (its `span_local` boundaries are unix timestamps of local times).
    let as_utc = local.assume_utc();
    let offset = match zone.get_offset_local(&as_utc) {
        OffsetResult::Some(offset) => offset,
        // Ambiguous local times happen when clocks fall back; pick the first
        // (pre-transition) offset, matching common calendar behaviour.
        OffsetResult::Ambiguous(offset, _) => offset,
        OffsetResult::None => {
            return Err(Error::Other(format!(
                "Local time does not exist in {zone_name} because of a DST transition"
            )))
        }
    };
    Ok(local.assume_offset(offset.to_utc()))
}

const CONFLICT: &str = "Provide either explicit UTC offsets or a timezone, not both";
const NEEDS_ZONE: &str =
    "Timestamps without an offset need a timezone (e.g. \"timezone\": \"auto\")";

/// Resolves `create_event` timestamps. Backward compatible: an explicit offset
/// (including `Z`) is stored verbatim, exactly as before. A floating local time
/// requires a `timezone`, which is used to compute the offset. Returns the
/// resolved instants plus the zone that was applied (echoed in the response).
pub fn resolve_create_times(
    starts_at: Option<EventTime>,
    ends_at: Option<EventTime>,
    timezone: Option<&str>,
    lat: f64,
    lon: f64,
) -> Result<(
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<String>,
)> {
    let has_local = [starts_at, ends_at]
        .iter()
        .any(|it| matches!(it, Some(EventTime::Local(_))));
    let has_absolute = [starts_at, ends_at]
        .iter()
        .any(|it| matches!(it, Some(EventTime::Absolute(_))));

    if has_local && has_absolute {
        return Err(Error::Other(CONFLICT.into()));
    }
    if has_local && timezone.is_none() {
        return Err(Error::Other(NEEDS_ZONE.into()));
    }
    if has_absolute && timezone.is_some() {
        return Err(Error::Other(CONFLICT.into()));
    }

    if has_local {
        let zone = resolve_zone_name(timezone.unwrap(), lat, lon)?;
        let resolve = |it: Option<EventTime>| -> Result<Option<OffsetDateTime>> {
            match it {
                Some(EventTime::Local(local)) => Ok(Some(local_to_utc(&zone, local)?)),
                _ => Ok(None),
            }
        };
        return Ok((resolve(starts_at)?, resolve(ends_at)?, Some(zone)));
    }

    let absolute = |it: Option<EventTime>| match it {
        Some(EventTime::Absolute(datetime)) => Some(datetime),
        _ => None,
    };
    Ok((absolute(starts_at), absolute(ends_at), None))
}

/// Resolves `update_event` timestamps. Each field is tri-state: `None` leaves it
/// untouched, `Some(None)` clears it, `Some(Some(value))` sets it. Since the
/// zone is not persisted, floating timestamps rely on the `timezone` provided in
/// this request. Returns the resolved fields plus the zone used, if any.
pub fn resolve_update_times(
    starts_at: FieldUpdate<EventTime>,
    ends_at: FieldUpdate<EventTime>,
    timezone: Option<&str>,
    lat: f64,
    lon: f64,
) -> Result<(
    FieldUpdate<OffsetDateTime>,
    FieldUpdate<OffsetDateTime>,
    Option<String>,
)> {
    let values = [&starts_at, &ends_at];
    let has_local = values
        .iter()
        .any(|it| matches!(it, Some(Some(EventTime::Local(_)))));
    let has_absolute = values
        .iter()
        .any(|it| matches!(it, Some(Some(EventTime::Absolute(_)))));

    if has_local && has_absolute {
        return Err(Error::Other(CONFLICT.into()));
    }

    let zone = if has_local {
        let spec = timezone.ok_or_else(|| Error::Other(NEEDS_ZONE.into()))?;
        Some(resolve_zone_name(spec, lat, lon)?)
    } else {
        None
    };

    let resolve = |it: Option<Option<EventTime>>| -> Result<Option<Option<OffsetDateTime>>> {
        match it {
            None => Ok(None),
            Some(None) => Ok(Some(None)),
            Some(Some(EventTime::Absolute(datetime))) => Ok(Some(Some(datetime))),
            Some(Some(EventTime::Local(local))) => {
                let zone = zone
                    .as_deref()
                    .ok_or_else(|| Error::Other(NEEDS_ZONE.into()))?;
                Ok(Some(Some(local_to_utc(zone, local)?)))
            }
        }
    };
    Ok((resolve(starts_at)?, resolve(ends_at)?, zone))
}

#[cfg(test)]
mod test {
    use super::{resolve_create_times, resolve_update_times, EventTime};
    use crate::Result;
    use time::macros::datetime;
    use time::{Date, Month, PrimitiveDateTime, Time};

    fn local(hour: u8) -> EventTime {
        EventTime::Local(PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 15).unwrap(),
            Time::from_hms(hour, 0, 0).unwrap(),
        ))
    }

    #[test]
    fn absolute_timestamps_are_untouched() -> Result<()> {
        let input = EventTime::Absolute(datetime!(2026-07-15 19:00 +07:00));
        let (starts, ends, zone) = resolve_create_times(Some(input), None, None, 13.75, 100.5)?;
        assert_eq!(Some(datetime!(2026-07-15 19:00 +07:00)), starts);
        assert_eq!(None, ends);
        assert_eq!(None, zone);
        Ok(())
    }

    #[test]
    fn auto_infers_zone_from_coordinates() -> Result<()> {
        // Bangkok in July is UTC+07:00.
        let (starts, _, zone) =
            resolve_create_times(Some(local(19)), None, Some("auto"), 13.75, 100.5)?;
        assert_eq!(Some(datetime!(2026-07-15 19:00 +07:00)), starts);
        assert_eq!(Some("Asia/Bangkok".to_string()), zone);
        Ok(())
    }

    #[test]
    fn named_zone_handles_dst() -> Result<()> {
        // Berlin in July is UTC+02:00 (CEST).
        let (starts, _, zone) =
            resolve_create_times(Some(local(19)), None, Some("Europe/Berlin"), 52.5, 13.4)?;
        assert_eq!(Some(datetime!(2026-07-15 19:00 +02:00)), starts);
        assert_eq!(Some("Europe/Berlin".to_string()), zone);
        Ok(())
    }

    #[test]
    fn floating_without_zone_is_rejected() {
        assert!(resolve_create_times(Some(local(19)), None, None, 13.75, 100.5).is_err());
    }

    #[test]
    fn absolute_with_zone_is_rejected() {
        let input = EventTime::Absolute(datetime!(2026-07-15 19:00 +07:00));
        assert!(resolve_create_times(Some(input), None, Some("auto"), 13.75, 100.5).is_err());
    }

    #[test]
    fn unknown_zone_is_rejected() {
        assert!(
            resolve_create_times(Some(local(19)), None, Some("Mars/Olympus"), 13.75, 100.5)
                .is_err()
        );
    }

    #[test]
    fn update_clears_and_keeps_fields() -> Result<()> {
        let (starts, ends, zone) = resolve_update_times(None, Some(None), None, 13.75, 100.5)?;
        assert_eq!(None, starts);
        assert_eq!(Some(None), ends);
        assert_eq!(None, zone);
        Ok(())
    }

    #[test]
    fn update_resolves_floating_time_with_zone() -> Result<()> {
        let (starts, _, zone) = resolve_update_times(
            Some(Some(local(19))),
            None,
            Some("Europe/Berlin"),
            52.5,
            13.4,
        )?;
        assert_eq!(Some(Some(datetime!(2026-07-15 19:00 +02:00))), starts);
        assert_eq!(Some("Europe/Berlin".to_string()), zone);
        Ok(())
    }

    #[test]
    fn parses_floating_local_string() {
        let value: EventTime = serde_json::from_str("\"2026-08-20T19:00:00\"").unwrap();
        assert!(matches!(value, EventTime::Local(_)));
    }

    #[test]
    fn parses_rfc3339_string() {
        let value: EventTime = serde_json::from_str("\"2026-08-20T19:00:00Z\"").unwrap();
        assert_eq!(EventTime::Absolute(datetime!(2026-08-20 19:00 UTC)), value);
    }
}
