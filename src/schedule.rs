//! Minimal cron expression parser and matcher used by `schedule` triggers.
//!
//! Supports the standard five fields: minute, hour, day of month, month and
//! day of week. Each field accepts `*`, `*/step`, lists, ranges and range
//! steps (e.g. `1-5/2`). Names such as `MON` are intentionally not supported.

use crate::parser::RuleTrigger;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Local, Timelike};
use std::thread;
use std::time::Duration;

/// A rule with an active cron trigger, as presented by `bee schedule list`.
pub struct ScheduledRule {
    pub name: String,
    pub task: String,
    pub cron: String,
}

#[derive(Debug, Clone, Copy)]
struct Field {
    min: u32,
    max: u32,
}

const MINUTE: Field = Field { min: 0, max: 59 };
const HOUR: Field = Field { min: 0, max: 23 };
const DAY: Field = Field { min: 1, max: 31 };
const MONTH: Field = Field { min: 1, max: 12 };
const WEEKDAY: Field = Field { min: 0, max: 7 };

#[derive(Debug, Clone)]
struct Cron {
    fields: [Vec<u32>; 5],
    dom_restricted: bool,
    dow_restricted: bool,
}

pub fn validate_expression(expression: &str) -> Result<()> {
    parse(expression).map(|_| ())
}

pub fn matches_expression(expression: &str, at: &DateTime<Local>) -> Result<bool> {
    let cron = parse(expression)?;
    let minute = at.minute();
    let hour = at.hour();
    let day = at.day();
    let month = at.month();
    // chrono uses 0..=6 for weekday (Monday = 0); cron uses 0..=7 (Sunday = 0/7).
    let weekday = (at.weekday().num_days_from_sunday()) % 7;

    if !cron.fields[0].contains(&minute) || !cron.fields[1].contains(&hour) {
        return Ok(false);
    }
    if !cron.fields[3].contains(&month) {
        return Ok(false);
    }

    let dom = cron.fields[2].contains(&day);
    let dow = cron.fields[4].contains(&weekday) || (weekday == 0 && cron.fields[4].contains(&7));
    let day_matches = match (cron.dom_restricted, cron.dow_restricted) {
        (true, true) => dom || dow,
        _ => dom && dow,
    };

    Ok(day_matches)
}

pub fn next_occurrence(expression: &str, from: &DateTime<Local>) -> Result<DateTime<Local>> {
    let cron = parse(expression)?;
    let mut candidate = from.with_second(0).and_then(|d| d.with_nanosecond(0));
    let Some(mut candidate) = candidate.take() else {
        return Err(anyhow!("could not normalize cron start time"));
    };
    // Scan minute by minute. A year's worth of minutes is a hard bound and
    // keeps a malformed-but-parseable expression from looping forever.
    for _ in 0..(366 * 24 * 60) {
        candidate += chrono::Duration::minutes(1);
        if cron_matches(&cron, &candidate) {
            return Ok(candidate);
        }
    }
    Err(anyhow!(
        "cron expression '{}' has no occurrence within a year",
        expression
    ))
}

fn cron_matches(cron: &Cron, at: &DateTime<Local>) -> bool {
    let minute = at.minute();
    let hour = at.hour();
    let day = at.day();
    let month = at.month();
    let weekday = at.weekday().num_days_from_sunday() % 7;
    if !cron.fields[0].contains(&minute)
        || !cron.fields[1].contains(&hour)
        || !cron.fields[3].contains(&month)
    {
        return false;
    }
    let dom = cron.fields[2].contains(&day);
    let dow = cron.fields[4].contains(&weekday) || (weekday == 0 && cron.fields[4].contains(&7));
    match (cron.dom_restricted, cron.dow_restricted) {
        (true, true) => dom || dow,
        _ => dom && dow,
    }
}

fn parse(expression: &str) -> Result<Cron> {
    let parts: Vec<&str> = expression.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(anyhow!(
            "invalid cron expression '{}': expected 5 fields (minute hour day month weekday)",
            expression
        ));
    }
    let fields = [
        parse_field(parts[0], MINUTE)?,
        parse_field(parts[1], HOUR)?,
        parse_field(parts[2], DAY)?,
        parse_field(parts[3], MONTH)?,
        parse_field(parts[4], WEEKDAY)?,
    ];
    Ok(Cron {
        fields,
        dom_restricted: parts[2] != "*",
        dow_restricted: parts[4] != "*",
    })
}

fn parse_field(part: &str, field: Field) -> Result<Vec<u32>> {
    let mut values = Vec::new();
    for item in part.split(',') {
        if item.is_empty() {
            return Err(anyhow!("invalid cron field '{}': empty item", part));
        }
        let (range, step) = match item.split_once('/') {
            Some((range, step)) => {
                let step: u32 = step.parse().map_err(|_| {
                    anyhow!("invalid cron field '{}': step must be a number", part)
                })?;
                if step == 0 {
                    return Err(anyhow!("invalid cron field '{}': step cannot be zero", part));
                }
                (range, step)
            }
            None => (item, 1),
        };

        let (start, end) = if range == "*" {
            (field.min, field.max)
        } else if let Some((start, end)) = range.split_once('-') {
            let start = parse_number(start, field, part)?;
            let end = parse_number(end, field, part)?;
            if start > end {
                return Err(anyhow!(
                    "invalid cron field '{}': range start is greater than end",
                    part
                ));
            }
            (start, end)
        } else {
            let value = parse_number(range, field, part)?;
            (value, value)
        };

        for value in (start..=end).step_by(step as usize) {
            values.push(value);
        }
    }
    values.sort_unstable();
    values.dedup();
    Ok(values)
}

fn parse_number(value: &str, field: Field, part: &str) -> Result<u32> {
    let number: u32 = value
        .parse()
        .map_err(|_| anyhow!("invalid cron field '{}': '{}' is not a number", part, value))?;
    if number < field.min || number > field.max {
        return Err(anyhow!(
            "invalid cron field '{}': {} is outside {}-{}",
            part,
            number,
            field.min,
            field.max
        ));
    }
    Ok(number)
}

pub fn now_local() -> DateTime<Local> {
    Local::now()
}

/// All registered rules that contain at least one cron trigger.
pub fn scheduled_rules() -> Result<Vec<ScheduledRule>> {
    let mut result = Vec::new();
    for rule in crate::parser::reader::config::read_rules()? {
        for trigger in &rule.triggers {
            if let RuleTrigger::Schedule { cron } = trigger {
                // Re-validate here: a hand-edited file must never reach the daemon.
                validate_expression(cron)?;
                result.push(ScheduledRule {
                    name: rule.name.clone(),
                    task: rule.task.clone(),
                    cron: cron.clone(),
                });
            }
        }
    }
    Ok(result)
}

pub fn list_scheduled() -> Result<()> {
    let now = now_local();
    let rules = scheduled_rules()?;
    if crate::ui::json_mode() {
        let mut items = json::Array::new();
        for rule in &rules {
            let next = next_occurrence(&rule.cron, &now)?;
            items.push(json::object! {
                "name" => rule.name.clone(),
                "task" => rule.task.clone(),
                "cron" => rule.cron.clone(),
                "next_run" => next.to_rfc3339(),
            });
        }
        crate::ui::print_json(&json::object! { "scheduled" => items });
        return Ok(());
    }

    if rules.is_empty() {
        println!("No scheduled rules. Add one with: bee rule modify <name> --trigger-type schedule --trigger-param cron='*/5 * * * *'");
        return Ok(());
    }
    println!("Scheduled rules:");
    for rule in &rules {
        let next = next_occurrence(&rule.cron, &now)?;
        println!(
            "  {}: task '{}', cron '{}', next {}",
            rule.name,
            rule.task,
            rule.cron,
            next.format("%Y-%m-%d %H:%M:%S")
        );
    }
    Ok(())
}

/// Run one scheduling pass. Returns the number of rules that fired.
pub fn run_pass(now: &DateTime<Local>) -> Result<usize> {
    let mut fired = 0;
    for rule in scheduled_rules()? {
        if !matches_expression(&rule.cron, now)? {
            continue;
        }
        // Fire at most once per rule per minute, including across restarts.
        let key = format!(
            "schedule_fired_{}_{}",
            rule.name,
            now.format("%Y-%m-%dT%H:%M")
        );
        if crate::run::cache::get_state(&key).is_some() {
            continue;
        }

        if crate::ui::dry_run() {
            crate::ui::info(format!(
                "[dry-run] would run task '{}' for rule '{}' (cron '{}')",
                rule.task, rule.name, rule.cron
            ));
        } else {
            crate::ui::info(format!(
                "schedule: firing task '{}' for rule '{}'",
                rule.task, rule.name
            ));
            match crate::run::run_task(rule.task.clone(), true, false) {
                Ok(()) => {
                    crate::ui::info(format!("schedule: task '{}' completed", rule.task))
                }
                Err(err) => crate::ui::einfo(format!(
                    "schedule: task '{}' failed: {}",
                    rule.task, err
                )),
            }
            crate::run::cache::save_state_final(&key, &now.to_rfc3339())?;
        }
        fired += 1;
    }
    Ok(fired)
}

/// Foreground scheduler daemon. With `once` it performs a single pass and
/// exits, which is also useful for cron-driven one-shot invocations.
pub fn run_daemon(interval: u64, once: bool) -> Result<()> {
    if interval == 0 {
        return Err(anyhow!("schedule run: --interval must be at least 1 second"));
    }
    let rules = scheduled_rules()?;
    if rules.is_empty() && !once {
        println!("No scheduled rules registered; the daemon will keep watching for changes.");
    }
    if !once {
        crate::ui::info(format!(
            "Scheduler started with {} rule(s), polling every {}s (Ctrl-C to stop).",
            rules.len(),
            interval
        ));
    }
    loop {
        let now = now_local();
        run_pass(&now)?;
        if once {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(interval));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_common_expressions() {
        validate_expression("0 0 * * *").unwrap();
        validate_expression("*/15 9-17 * * 1-5").unwrap();
        assert!(validate_expression("60 * * * *").is_err());
        assert!(validate_expression("bad").is_err());
        assert!(validate_expression("* * * *").is_err());
    }

    #[test]
    fn matches_fields() {
        let at = Local::now()
            .with_year(2026)
            .and_then(|d| d.with_month(11))
            .and_then(|d| d.with_day(27))
            .and_then(|d| d.with_hour(9))
            .and_then(|d| d.with_minute(30))
            .and_then(|d| d.with_second(0))
            .unwrap();
        assert!(matches_expression("30 9 27 11 *", &at).unwrap());
        assert!(!matches_expression("31 9 27 11 *", &at).unwrap());
        assert!(matches_expression("*/15 9 * * *", &at).unwrap());
    }

    #[test]
    fn finds_next_occurrence() {
        let at = Local::now()
            .with_year(2026)
            .and_then(|d| d.with_month(11))
            .and_then(|d| d.with_day(27))
            .and_then(|d| d.with_hour(9))
            .and_then(|d| d.with_minute(31))
            .and_then(|d| d.with_second(0))
            .unwrap();
        let next = next_occurrence("*/15 * * * *", &at).unwrap();
        assert_eq!(next.minute(), 45);
    }
}
