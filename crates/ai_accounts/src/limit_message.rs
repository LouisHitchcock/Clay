//! Extracting a reset time from an agent's usage-limit message.
//!
//! ACP has no structured signal for a usage limit — `StopReason` covers `EndTurn`, `MaxTokens`,
//! `MaxTurnRequests`, `Refusal` and `Cancelled`, and nothing else — so when Claude Code stops
//! for one, all Clay gets is the text the CLI printed. This reads a reset time out of that.
//!
//! Pattern-matching prose is inherently fragile: Anthropic can reword the message in any CLI
//! release, and the wording is **not verified against a real limit message yet**. Everything
//! here is therefore best-effort, and a failure to parse returns `None` so the caller falls back
//! to a whole window rather than dropping the resume entirely. A late resume is recoverable; a
//! lost one is not.

use chrono::{DateTime, Duration, FixedOffset, NaiveTime, TimeZone, Utc};

/// Whether a message looks like the agent refusing to continue because of a usage limit.
///
/// Kept separate from parsing the time, because a limit with an unreadable time still needs to
/// be treated as a limit.
pub fn looks_like_usage_limit(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    let mentions_limit = message.contains("usage limit")
        || message.contains("rate limit")
        || message.contains("limit reached")
        || message.contains("limit hit")
        || message.contains("out of credits")
        || message.contains("credit balance");
    // "limit" on its own catches far too much — tool-call limits, token limits, and any prose
    // the model happens to write about limits.
    mentions_limit
}

/// Reads the moment a limit lifts out of `message`, if it says.
///
/// `offset` is the zone a bare clock time should be read in — the user's local zone in practice.
/// A message that names a zone is still read in `offset`, because mapping zone abbreviations is
/// its own can of worms and being an hour out is better than refusing to resume.
pub fn parse_reset_time(
    message: &str,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> Option<DateTime<Utc>> {
    // Some CLI versions append a Unix timestamp, which needs no interpretation at all.
    if let Some(at) = parse_unix_timestamp(message, now) {
        return Some(at);
    }
    parse_clock_time(message, now, offset)
}

/// A bare 10-digit Unix timestamp, as `Claude AI usage limit reached|1750000000` does.
fn parse_unix_timestamp(message: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let mut best: Option<DateTime<Utc>> = None;
    for run in message.split(|c: char| !c.is_ascii_digit()) {
        // Ten digits is a second-precision timestamp for any date this side of 2286. Shorter
        // runs are clock times and version numbers; longer ones are milliseconds, which are not
        // a shape Claude Code has been seen to emit.
        if run.len() != 10 {
            continue;
        }
        let Ok(seconds) = run.parse::<i64>() else {
            continue;
        };
        let Some(at) = DateTime::from_timestamp(seconds, 0) else {
            continue;
        };
        // Only a plausible reset: in the future, and not implausibly far into it. A stray
        // ten-digit number elsewhere in the message would otherwise schedule a resume in 2030.
        if at > now && at < now + Duration::days(14) && best.is_none_or(|best| at < best) {
            best = Some(at);
        }
    }
    best
}

/// A clock time such as `3pm`, `3:30 pm`, `15:00`, resolved to its next occurrence.
fn parse_clock_time(
    message: &str,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> Option<DateTime<Utc>> {
    let lowered = message.to_ascii_lowercase();
    // Anchored on "reset" so an unrelated time in the message is not mistaken for one. Claude
    // Code's phrasings all put the time after that word: "resets at", "will reset at", "reset
    // at".
    let after_reset = lowered.split("reset").nth(1)?;
    next_occurrence_of_clock_time(after_reset, now, offset)
}

/// The next time `text`'s clock time comes round, in `offset`.
///
/// Shared with the manual scheduler, where the user types a time directly rather than it being
/// quoted back by an agent.
pub fn next_occurrence_of_clock_time(
    text: &str,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> Option<DateTime<Utc>> {
    let lowered = text.to_ascii_lowercase();
    let time = find_clock_time(&lowered)?;

    let local_now = now.with_timezone(&offset);
    let today = local_now.date_naive();
    for day in 0..=1 {
        let candidate = today + Duration::days(day);
        // `from_local_datetime` is ambiguous across a DST fold; either instant is within an hour
        // of the intended one, which does not matter for waiting out a five-hour window.
        let Some(local) = offset.from_local_datetime(&candidate.and_time(time)).earliest() else {
            continue;
        };
        let candidate = local.with_timezone(&Utc);
        if candidate > now {
            return Some(candidate);
        }
    }
    None
}

/// The first `h`, `h:mm`, `hpm`, `h:mm pm` in `text`.
fn find_clock_time(text: &str) -> Option<NaiveTime> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let hour_text = &text[start..i];
        // More than two digits is a year, a count or a timestamp, not an hour.
        if hour_text.len() > 2 {
            continue;
        }
        let Ok(hour) = hour_text.parse::<u32>() else {
            continue;
        };

        let mut minute = 0;
        if i < bytes.len() && bytes[i] == b':' {
            let minute_start = i + 1;
            let mut j = minute_start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j - minute_start != 2 {
                continue;
            }
            let Ok(parsed) = text[minute_start..j].parse::<u32>() else {
                continue;
            };
            minute = parsed;
            i = j;
        }

        let rest = text[i..].trim_start();
        let hour = if rest.starts_with("pm") {
            // 12pm is noon, not hour 24.
            if hour == 12 { 12 } else { hour + 12 }
        } else if rest.starts_with("am") {
            // 12am is midnight.
            if hour == 12 { 0 } else { hour }
        } else if hour_text.len() == 2 || minute > 0 {
            // A 24-hour clock time: "15:00", or "09" in "resets at 09:00".
            hour
        } else {
            // A bare single digit with no meridiem is not a time we can trust.
            continue;
        };

        if let Some(time) = NaiveTime::from_hms_opt(hour, minute, 0) {
            return Some(time);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    fn london() -> FixedOffset {
        FixedOffset::east_opt(60 * 60).unwrap()
    }

    #[test]
    fn recognises_limit_messages() {
        assert!(looks_like_usage_limit("Claude usage limit reached"));
        assert!(looks_like_usage_limit("You have hit your rate limit"));
        assert!(looks_like_usage_limit("limit hit, resets at 3pm"));
        assert!(looks_like_usage_limit(
            "Your credit balance is too low to continue"
        ));
    }

    #[test]
    fn does_not_mistake_other_limits_for_a_usage_limit() {
        // These stop a turn too, but waiting for a reset would never help.
        assert!(!looks_like_usage_limit(
            "The conversation exceeded the maximum token count"
        ));
        assert!(!looks_like_usage_limit("Reached the tool call limit"));
        assert!(!looks_like_usage_limit("I cannot help with that"));
    }

    #[test]
    fn reads_a_unix_timestamp() {
        let now = utc(2026, 9, 4, 12, 0);
        let reset = utc(2026, 9, 4, 17, 0);
        let message = format!("Claude AI usage limit reached|{}", reset.timestamp());
        assert_eq!(parse_reset_time(&message, now, london()), Some(reset));
    }

    #[test]
    fn ignores_a_timestamp_in_the_past() {
        // A stale message replayed from history must not schedule a resume that fires at once.
        let now = utc(2026, 9, 4, 12, 0);
        let past = utc(2026, 9, 4, 6, 0);
        let message = format!("usage limit reached|{}", past.timestamp());
        assert_eq!(parse_reset_time(&message, now, london()), None);
    }

    #[test]
    fn reads_a_twelve_hour_clock_time() {
        let now = utc(2026, 9, 4, 12, 0);
        // 15:00 London is 14:00 UTC.
        assert_eq!(
            parse_reset_time("limit hit, resets at 3pm", now, london()),
            Some(utc(2026, 9, 4, 14, 0))
        );
    }

    #[test]
    fn reads_minutes_too() {
        let now = utc(2026, 9, 4, 12, 0);
        assert_eq!(
            parse_reset_time("Your limit will reset at 3:30pm", now, london()),
            Some(utc(2026, 9, 4, 14, 30))
        );
    }

    #[test]
    fn reads_a_twenty_four_hour_clock_time() {
        let now = utc(2026, 9, 4, 12, 0);
        assert_eq!(
            parse_reset_time("resets at 15:00", now, london()),
            Some(utc(2026, 9, 4, 14, 0))
        );
    }

    #[test]
    fn a_time_already_past_today_means_tomorrow() {
        // 09:00 London is 08:00 UTC, which has been and gone.
        let now = utc(2026, 9, 4, 12, 0);
        assert_eq!(
            parse_reset_time("resets at 9am", now, london()),
            Some(utc(2026, 9, 5, 8, 0))
        );
    }

    #[test]
    fn ignores_times_that_are_not_the_reset_time() {
        // The 10:00 belongs to the sentence before "reset", so it must not be picked up.
        let now = utc(2026, 9, 4, 12, 0);
        let message = "You started at 10:00 and hit the limit";
        assert_eq!(parse_reset_time(message, now, london()), None);
    }

    #[test]
    fn a_message_with_no_time_gives_nothing() {
        let now = utc(2026, 9, 4, 12, 0);
        assert_eq!(
            parse_reset_time("Claude usage limit reached", now, london()),
            None
        );
        assert_eq!(parse_reset_time("", now, london()), None);
    }

    #[test]
    fn a_zone_in_the_message_is_read_in_the_local_offset() {
        // Documented shortcoming: the named zone is ignored rather than mapped, so this is read
        // as 15:00 local. An hour out is acceptable when waiting on a five-hour window.
        let now = utc(2026, 9, 4, 12, 0);
        assert_eq!(
            parse_reset_time("resets at 3pm (America/New_York)", now, london()),
            Some(utc(2026, 9, 4, 14, 0))
        );
    }
}
