use std::str::FromStr;

use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Paris;
use icalendar::{CalendarDateTime, Calendar, CalendarComponent, Component, DatePerhapsTime};

use crate::calendar::state::State;

pub async fn run(state: &State, ical_url: &str, room_id: i32) {
    tracing::info!(room_id, "Google Calendar sync: starting");

    let body = match reqwest::get(ical_url).await {
        Ok(resp) => match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "Google Calendar sync: failed to read response body");
                return;
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "Google Calendar sync: failed to fetch iCal URL");
            return;
        }
    };

    let calendar = match Calendar::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Google Calendar sync: failed to parse iCal feed");
            return;
        }
    };

    let mut inserted = 0usize;
    let mut skipped = 0usize;

    for component in calendar.iter() {
        let CalendarComponent::Event(event) = component else {
            continue;
        };

        let uid = match event.get_uid() {
            Some(u) => u.to_string(),
            None => {
                tracing::warn!("Google Calendar sync: event without UID, skipping");
                continue;
            }
        };

        let title = event.get_summary().unwrap_or("(sans titre)").to_string();
        let notes = event.get_description().unwrap_or("").to_string();

        let Some(start) = event.get_start().and_then(to_utc) else {
            tracing::warn!(uid, "Google Calendar sync: event has no parseable start, skipping");
            continue;
        };
        let Some(end) = event.get_end().and_then(to_utc) else {
            tracing::warn!(uid, "Google Calendar sync: event has no parseable end, skipping");
            continue;
        };

        let result = sqlx::query(
            "INSERT INTO portal_room_booking (room_id, title, start_at, end_at, notes, google_uid) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (google_uid) DO NOTHING",
        )
        .bind(room_id)
        .bind(&title)
        .bind(start)
        .bind(end)
        .bind(&notes)
        .bind(&uid)
        .execute(&state.db)
        .await;

        match result {
            Ok(r) if r.rows_affected() == 1 => inserted += 1,
            Ok(_) => skipped += 1,
            Err(e) => tracing::error!(uid, error = %e, "Google Calendar sync: DB insert failed"),
        }
    }

    tracing::info!(room_id, inserted, skipped, "Google Calendar sync: done");
}

fn to_utc(d: DatePerhapsTime) -> Option<DateTime<Utc>> {
    match d {
        DatePerhapsTime::DateTime(CalendarDateTime::Utc(dt)) => Some(dt),
        DatePerhapsTime::DateTime(CalendarDateTime::Floating(ndt)) => Paris
            .from_local_datetime(&ndt)
            .single()
            .map(|dt| dt.with_timezone(&Utc)),
        DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { date_time, tzid }) => tzid
            .parse::<chrono_tz::Tz>()
            .ok()
            .and_then(|tz| tz.from_local_datetime(&date_time).single())
            .map(|dt| dt.with_timezone(&Utc)),
        DatePerhapsTime::Date(nd) => Paris
            .from_local_datetime(&nd.and_time(NaiveTime::MIN))
            .single()
            .map(|dt| dt.with_timezone(&Utc)),
    }
}
