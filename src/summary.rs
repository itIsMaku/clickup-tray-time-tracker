use crate::locale::{Key, Message};
use crate::model::TimeEntry;
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Goals {
    pub month_hours: f64,
    pub week_hours: f64,
}
impl Default for Goals {
    fn default() -> Self {
        Self {
            month_hours: 80.,
            week_hours: 20.,
        }
    }
}
impl Goals {
    pub fn parse(month: &str, week: &str) -> Result<Self> {
        fn hours(input: &str) -> Result<f64> {
            let value: f64 = input
                .trim()
                .replace(',', ".")
                .parse()
                .map_err(|_| anyhow::anyhow!(Message::new(Key::GoalNumber)))?;
            anyhow::ensure!(
                value.is_finite() && value > 0. && value <= 10000.,
                Message::new(Key::GoalRange)
            );
            Ok(value)
        }
        Ok(Self {
            month_hours: hours(month)?,
            week_hours: hours(week)?,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Periods {
    pub month_start: i64,
    pub week_start: i64,
}
impl Periods {
    pub fn at<Tz: TimeZone>(now: DateTime<Tz>) -> Result<Self> {
        let timezone = now.timezone();
        let start = |date: NaiveDate| -> Result<i64> {
            // A few time zones move their clocks at midnight. Use the first
            // existing local hour instead of assuming every day starts at 00:00.
            for hour in 0..=3 {
                if let Some(local) = timezone
                    .from_local_datetime(&date.and_hms_opt(hour, 0, 0).unwrap())
                    .earliest()
                {
                    return Ok(local.timestamp_millis());
                }
            }
            anyhow::bail!(Message::new(Key::PeriodStart))
        };
        let today = now.date_naive();
        Ok(Self {
            month_start: start(today.with_day(1).unwrap())?,
            week_start: start(
                today - Duration::days(today.weekday().num_days_from_monday() as i64),
            )?,
        })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub entries: Vec<TimeEntry>,
    pub fetched_at: i64,
}
#[derive(Default, Debug)]
pub struct Totals {
    pub month_ms: i64,
    pub week_ms: i64,
}
impl Summary {
    pub fn totals(&self, current: Option<&TimeEntry>, periods: &Periods, now: i64) -> Totals {
        let mut totals = Totals::default();
        let mut seen = std::collections::HashSet::new();
        let mut add = |entry: &TimeEntry, cutoff: i64| {
            if !seen.insert(entry.id.clone()) {
                return;
            }
            let Ok(start) = entry.start.parse::<i64>() else {
                return;
            };
            let end = start.saturating_add(entry.elapsed_ms(cutoff)).min(cutoff);
            totals.month_ms += end.saturating_sub(start.max(periods.month_start)).max(0);
            totals.week_ms += end.saturating_sub(start.max(periods.week_start)).max(0);
        };
        // A completed entry from the newer history response supersedes an old
        // /current snapshot taken just before an external stop or switch.
        let current = current.filter(|current| {
            !self.entries.iter().any(|entry| {
                entry.id == current.id
                    && entry
                        .duration
                        .parse::<i64>()
                        .is_ok_and(|duration| duration >= 0)
            })
        });
        if let Some(entry) = current {
            add(entry, now);
        }
        for entry in &self.entries {
            let mut cutoff = self.fetched_at.min(now);
            if entry
                .duration
                .parse::<i64>()
                .is_ok_and(|duration| duration < 0)
                && let Some(start) = current.and_then(|entry| entry.start.parse::<i64>().ok())
            {
                cutoff = cutoff.min(start);
            }
            add(entry, cutoff);
        }
        totals
    }
}
pub struct GoalProgress {
    pub percent: f32,
    pub remaining_ms: i64,
    pub extra_ms: i64,
}
impl GoalProgress {
    pub fn new(tracked: i64, goal_hours: f64) -> Self {
        let goal = (goal_hours.max(0.) * 3600000.).round() as i64;
        Self {
            percent: if goal > 0 {
                (100. * tracked.max(0) as f64 / goal as f64).clamp(0., 100.) as f32
            } else {
                0.
            },
            remaining_ms: goal.saturating_sub(tracked).max(0),
            extra_ms: tracked.saturating_sub(goal).max(0),
        }
    }
}
pub fn hours_label(ms: i64) -> String {
    let minutes = ms.max(0) / 60000;
    format!("{} h {:02} min", minutes / 60, minutes % 60)
}
