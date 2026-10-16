use bld_models::dtos::CRON_USER;
use chrono::{NaiveDateTime, TimeDelta, Utc};
use croner::Cron;

pub fn parse_date(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f").ok()
}

pub fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

pub fn clock(delta: TimeDelta) -> String {
    let secs = delta.num_seconds().max(0);
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub fn run_duration(start: Option<&str>, end: Option<&str>) -> String {
    match (start.and_then(parse_date), end.and_then(parse_date)) {
        (Some(start), Some(end)) => clock(end - start),
        (Some(start), None) => format!("{}…", clock(now() - start)),
        _ => "—".to_string(),
    }
}

pub fn ago(value: &str) -> String {
    let Some(date) = parse_date(value) else {
        return String::new();
    };
    let delta = now() - date;
    let mins = delta.num_minutes();
    match mins {
        m if m < 1 => "just now".to_string(),
        m if m < 60 => format!("{m} min ago"),
        m if m < 60 * 24 => format!("{} h ago", m / 60),
        m if m < 60 * 48 => "yesterday".to_string(),
        m => format!("{} d ago", m / (60 * 24)),
    }
}

pub fn until(date: NaiveDateTime) -> String {
    let mins = (date - now()).num_minutes();
    match mins {
        m if m < 1 => "in <1 min".to_string(),
        m if m < 60 => format!("in {m} min"),
        m if m < 60 * 24 => format!("in {} h", m / 60),
        m => format!("in {} d", m / (60 * 24)),
    }
}

pub fn short_date(value: &str) -> String {
    parse_date(value)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| value.to_string())
}

pub fn short_id(id: &str) -> String {
    id.chars().take(18).collect()
}

pub fn trigger_label(user: &str) -> String {
    match user {
        CRON_USER => "cron".to_string(),
        "" => "manual".to_string(),
        user => user.to_string(),
    }
}

pub fn cron_next(expr: &str) -> Option<NaiveDateTime> {
    Cron::new(expr)
        .with_seconds_required()
        .with_dom_and_dow()
        .parse()
        .ok()?
        .find_next_occurrence(&Utc::now(), false)
        .ok()
        .map(|d| d.naive_utc())
}

fn day_name(value: &str) -> String {
    let name = match value {
        "0" | "7" => "Sun",
        "1" => "Mon",
        "2" => "Tue",
        "3" => "Wed",
        "4" => "Thu",
        "5" => "Fri",
        "6" => "Sat",
        "1-5" => "Mon-Fri",
        other => return other.to_uppercase(),
    };
    name.to_string()
}

pub fn cron_cadence(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() < 6 {
        return "Custom schedule".to_string();
    }
    let (sec, min, hour, dom, mon, dow) = (
        fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
    );
    let every = |v: &str| v.strip_prefix("*/").and_then(|n| n.parse::<u32>().ok());
    let number = |v: &str| v.parse::<u32>().ok();
    let any_day = dom == "*" && mon == "*" && dow == "*";

    if let Some(n) = every(sec)
        && min == "*"
        && hour == "*"
        && any_day
    {
        return format!("Every {n} seconds");
    }
    if sec != "0" {
        return "Custom schedule".to_string();
    }
    if hour == "*" && any_day {
        if min == "*" {
            return "Every minute".to_string();
        }
        if let Some(n) = every(min) {
            return format!("Every {n} minutes");
        }
        if let Some(m) = number(min) {
            return format!("Hourly at :{m:02}");
        }
    }
    let (Some(m), Some(h)) = (number(min), number(hour)) else {
        if let (Some(m), Some(n)) = (number(min), every(hour))
            && any_day
        {
            return format!("Every {n} hours at :{m:02}");
        }
        return "Custom schedule".to_string();
    };
    let time = format!("{h:02}:{m:02}");
    match (dom, mon, dow) {
        ("*", "*", "*") => format!("Daily at {time}"),
        ("*", "*", dow) => format!("{} at {time}", day_name(dow)),
        (dom, "*", "*") if number(dom).is_some() => format!("Monthly on day {dom} at {time}"),
        _ => "Custom schedule".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_cadence_describes_common_schedules() {
        assert_eq!(cron_cadence("0 */10 * * * *"), "Every 10 minutes");
        assert_eq!(cron_cadence("0 * * * * *"), "Every minute");
        assert_eq!(cron_cadence("*/30 * * * * *"), "Every 30 seconds");
        assert_eq!(cron_cadence("0 15 * * * *"), "Hourly at :15");
        assert_eq!(cron_cadence("0 0 */6 * * *"), "Every 6 hours at :00");
        assert_eq!(cron_cadence("0 0 3 * * *"), "Daily at 03:00");
        assert_eq!(cron_cadence("0 30 9 * * 1-5"), "Mon-Fri at 09:30");
        assert_eq!(cron_cadence("0 0 1 15 * *"), "Monthly on day 15 at 01:00");
        assert_eq!(cron_cadence("0 0 1 15 6 *"), "Custom schedule");
        assert_eq!(cron_cadence("bad"), "Custom schedule");
    }

    #[test]
    fn cron_next_parses_six_field_expressions() {
        assert!(cron_next("0 */10 * * * *").is_some());
        assert!(cron_next("not a cron").is_none());
    }

    #[test]
    fn clock_formats_minutes_and_hours() {
        assert_eq!(clock(TimeDelta::seconds(41)), "0:41");
        assert_eq!(clock(TimeDelta::seconds(3723)), "1:02:03");
        assert_eq!(clock(TimeDelta::seconds(-5)), "0:00");
    }

    #[test]
    fn run_duration_uses_start_and_end() {
        assert_eq!(
            run_duration(Some("2026-08-26 23:10:00"), Some("2026-08-26 23:10:12")),
            "0:12"
        );
        assert_eq!(run_duration(None, None), "—");
    }

    #[test]
    fn trigger_label_maps_cron_and_empty_users() {
        assert_eq!(trigger_label(CRON_USER), "cron");
        assert_eq!(trigger_label(""), "manual");
        assert_eq!(trigger_label("kostas"), "kostas");
    }
}
