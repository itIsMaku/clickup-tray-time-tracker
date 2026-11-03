use chrono::DateTime;
use clickup_tracker::{
    model::TimeEntry,
    summary::{GoalProgress, Goals, Periods, Summary, hours_label},
};

fn entry(id: &str, start: i64, duration: i64) -> TimeEntry {
    serde_json::from_value(
        serde_json::json!({"id":id,"start":start.to_string(),"duration":duration}),
    )
    .unwrap()
}

#[test]
fn period_starts_use_local_calendar_and_monday_even_across_months() {
    let now = DateTime::parse_from_rfc3339("2026-11-01T14:00:00+01:00").unwrap();
    let periods = Periods::at(now).unwrap();
    assert_eq!(
        periods.month_start,
        DateTime::parse_from_rfc3339("2026-11-01T00:00:00+01:00")
            .unwrap()
            .timestamp_millis()
    );
    assert_eq!(
        periods.week_start,
        DateTime::parse_from_rfc3339("2026-10-26T00:00:00+01:00")
            .unwrap()
            .timestamp_millis()
    );
}

#[test]
fn totals_clip_period_edges_and_count_a_running_timer_only_once() {
    let running = entry("live", 8000, -8000);
    let summary = Summary {
        entries: vec![
            entry("one", 500, 3000),
            entry("two", 4000, 2000),
            running.clone(),
        ],
        fetched_at: 9000,
    };
    let totals = summary.totals(
        Some(&running),
        &Periods {
            month_start: 1000,
            week_start: 3000,
        },
        10000,
    );
    assert_eq!(totals.month_ms, 6500);
    assert_eq!(totals.week_ms, 4500);
}

#[test]
fn old_running_history_stops_growing_after_current_has_changed() {
    let summary = Summary {
        entries: vec![entry("old", 2000, -2000)],
        fetched_at: 5000,
    };
    let totals = summary.totals(
        None,
        &Periods {
            month_start: 0,
            week_start: 0,
        },
        9000,
    );
    assert_eq!(totals.month_ms, 3000);
}

#[test]
fn newer_completed_history_wins_over_an_older_current_snapshot() {
    let summary = Summary {
        entries: vec![entry("a", 1000, 2000), entry("b", 3000, -3000)],
        fetched_at: 5000,
    };
    let stale_current = entry("a", 1000, -1000);
    assert_eq!(
        summary
            .totals(
                Some(&stale_current),
                &Periods {
                    month_start: 0,
                    week_start: 0
                },
                6000
            )
            .month_ms,
        4000
    );
}

#[test]
fn previous_running_history_is_capped_at_the_next_timer_start() {
    let summary = Summary {
        entries: vec![entry("a", 1000, -1000)],
        fetched_at: 5000,
    };
    let current = entry("b", 3000, -3000);
    assert_eq!(
        summary
            .totals(
                Some(&current),
                &Periods {
                    month_start: 0,
                    week_start: 0
                },
                6000
            )
            .month_ms,
        5000
    );
}

#[test]
fn goal_progress_clamps_bar_but_preserves_overtime() {
    let progress = GoalProgress::new(90 * 3600000, 80.);
    assert_eq!(progress.percent, 100.);
    assert_eq!(progress.remaining_ms, 0);
    assert_eq!(progress.extra_ms, 10 * 3600000);
    let progress = GoalProgress::new(5 * 3600000, 20.);
    assert_eq!(progress.percent, 25.);
    assert_eq!(progress.remaining_ms, 15 * 3600000);
    assert_eq!(hours_label(90 * 60000), "1 h 30 min");
}

#[test]
fn goal_input_accepts_decimal_comma_and_rejects_invalid_values() {
    let goals = Goals::parse("80", " 20,5 ").unwrap();
    assert_eq!(goals.week_hours, 20.5);
    for invalid in ["0", "-1", "NaN", "inf", "", "10001"] {
        assert!(Goals::parse(invalid, "20").is_err());
    }
}
