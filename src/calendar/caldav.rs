use fast_dav_rs::CalDavClient;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};
use uuid::Uuid;

use crate::error::AppError;

use super::{
    model::ParsedEvent,
    repository::{CalendarAccountSecrets, CalendarRepository},
};

pub async fn discover(
    base_url: &str,
    username: &str,
    password: &str,
) -> Result<Vec<fast_dav_rs::CalendarInfo>, AppError> {
    let client = CalDavClient::new(base_url, Some(username), Some(password))
        .map_err(|error| AppError::Calendar(error.to_string()))?;
    let principal = client
        .discover_current_user_principal()
        .await
        .map_err(|error| AppError::Calendar(error.to_string()))?
        .ok_or_else(|| AppError::Calendar("CalDAV principal was not found".into()))?;
    let homes = client
        .discover_calendar_home_set(&principal)
        .await
        .map_err(|error| AppError::Calendar(error.to_string()))?;
    let mut calendars = Vec::new();
    for home in homes {
        calendars.extend(
            client
                .list_calendars(&home)
                .await
                .map_err(|error| AppError::Calendar(error.to_string()))?,
        );
    }
    Ok(calendars)
}

pub async fn sync_account(
    repository: &CalendarRepository,
    user_id: Uuid,
    account_id: Uuid,
) -> Result<u32, AppError> {
    let (account, secrets) = repository
        .get_account_with_secrets(user_id, account_id)
        .await?;
    let remote_calendars =
        discover(&account.base_url, &account.username, secrets.password()).await?;
    repository
        .upsert_remote_calendars(user_id, account_id, remote_calendars)
        .await?;
    let client = client(&account.base_url, &account.username, &secrets)?;
    let local_calendars = repository
        .enabled_calendars_for_account(user_id, account_id)
        .await?;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let start = caldav_datetime(now.saturating_sub(90 * 86_400))?;
    let end = caldav_datetime(now.saturating_add(365 * 86_400))?;
    let mut imported = 0_u32;
    for calendar in local_calendars {
        let calendar_id = Uuid::parse_str(&calendar.id).map_err(AppError::internal)?;
        let objects = client
            .calendar_query_timerange(
                &calendar.remote_href,
                "VEVENT",
                Some(&start),
                Some(&end),
                true,
            )
            .await
            .map_err(|error| AppError::Calendar(error.to_string()))?;
        for object in objects {
            let Some(ics) = object.calendar_data else {
                continue;
            };
            for event in parse_ics_events(&ics) {
                repository
                    .upsert_event(
                        user_id,
                        calendar_id,
                        Some(object.href.clone()),
                        object.etag.clone(),
                        ics.clone(),
                        event,
                    )
                    .await?;
                imported += 1;
            }
        }
    }
    repository
        .mark_account_synced(user_id, account_id, None)
        .await?;
    Ok(imported)
}

fn client(
    base_url: &str,
    username: &str,
    secrets: &CalendarAccountSecrets,
) -> Result<CalDavClient, AppError> {
    CalDavClient::new(base_url, Some(username), Some(secrets.password()))
        .map_err(|error| AppError::Calendar(error.to_string()))
}

fn caldav_datetime(timestamp: i64) -> Result<String, AppError> {
    let value = OffsetDateTime::from_unix_timestamp(timestamp).map_err(AppError::internal)?;
    Ok(format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        value.year(),
        u8::from(value.month()),
        value.day(),
        value.hour(),
        value.minute(),
        value.second()
    ))
}

pub fn parse_ics_events(ics: &str) -> Vec<ParsedEvent> {
    let lines = unfold_lines(ics);
    let mut events = Vec::new();
    let mut current = Vec::new();
    let mut in_event = false;
    for line in lines {
        match line.as_str() {
            "BEGIN:VEVENT" => {
                in_event = true;
                current.clear();
            }
            "END:VEVENT" if in_event => {
                if let Some(event) = parse_event(&current) {
                    events.push(event);
                }
                in_event = false;
                current.clear();
            }
            _ if in_event => current.push(line),
            _ => {}
        }
    }
    events
}

fn unfold_lines(ics: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in ics.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(last) = lines.last_mut() {
                last.push_str(raw.trim_start());
            }
        } else if !raw.trim().is_empty() {
            lines.push(raw.trim_end().to_owned());
        }
    }
    lines
}

fn parse_event(lines: &[String]) -> Option<ParsedEvent> {
    let uid = property(lines, "UID").unwrap_or_else(|| Uuid::new_v4().to_string());
    let summary = property(lines, "SUMMARY").unwrap_or_else(|| "Untitled event".into());
    let description = property(lines, "DESCRIPTION").unwrap_or_default();
    let location = property(lines, "LOCATION").unwrap_or_default();
    let start_line = property_line(lines, "DTSTART")?;
    let end_line = property_line(lines, "DTEND");
    let start = parse_ics_datetime(&start_line)?;
    let mut end = end_line
        .as_deref()
        .and_then(parse_ics_datetime)
        .map(|value| value.timestamp)
        .unwrap_or_else(|| {
            if start.all_day {
                start.timestamp + 86_400
            } else {
                start.timestamp + 3600
            }
        });
    if end <= start.timestamp {
        end = start.timestamp + if start.all_day { 86_400 } else { 3600 };
    }
    Some(ParsedEvent {
        uid: unescape_text(&uid),
        summary: unescape_text(&summary),
        description: unescape_text(&description),
        location: unescape_text(&location),
        starts_at: start.timestamp,
        ends_at: end,
        all_day: start.all_day,
        timezone: start.timezone,
    })
}

fn property(lines: &[String], name: &str) -> Option<String> {
    property_line(lines, name)
        .and_then(|line| line.split_once(':').map(|(_, value)| value.to_owned()))
}

fn property_line(lines: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}:");
    let param_prefix = format!("{name};");
    lines
        .iter()
        .find(|line| line.starts_with(&prefix) || line.starts_with(&param_prefix))
        .cloned()
}

struct IcsDateTime {
    timestamp: i64,
    all_day: bool,
    timezone: Option<String>,
}

fn parse_ics_datetime(line: &str) -> Option<IcsDateTime> {
    let (head, value) = line.split_once(':')?;
    let all_day = head.contains("VALUE=DATE") || value.len() == 8;
    let timezone = head
        .split(';')
        .find_map(|part| part.strip_prefix("TZID="))
        .map(str::to_owned);
    let timestamp = if all_day {
        let date = parse_date(value)?;
        PrimitiveDateTime::new(date, Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp()
    } else {
        parse_datetime(value)?
    };
    Some(IcsDateTime {
        timestamp,
        all_day,
        timezone,
    })
}

fn parse_datetime(value: &str) -> Option<i64> {
    let value = value.trim_end_matches('Z');
    if value.len() < 15 {
        return None;
    }
    let date = parse_date(&value[..8])?;
    let hour = value[9..11].parse::<u8>().ok()?;
    let minute = value[11..13].parse::<u8>().ok()?;
    let second = value[13..15].parse::<u8>().ok()?;
    let time = Time::from_hms(hour, minute, second).ok()?;
    Some(
        PrimitiveDateTime::new(date, time)
            .assume_utc()
            .unix_timestamp(),
    )
}

fn parse_date(value: &str) -> Option<Date> {
    if value.len() != 8 {
        return None;
    }
    let year = value[0..4].parse::<i32>().ok()?;
    let month = Month::try_from(value[4..6].parse::<u8>().ok()?).ok()?;
    let day = value[6..8].parse::<u8>().ok()?;
    Date::from_calendar_date(year, month, day).ok()
}

fn unescape_text(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}
