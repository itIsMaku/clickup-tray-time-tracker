use crate::{State, send, worker::Command};
use clickup_tracker::locale::{Key, Message};
use clickup_tracker::summary::{GoalProgress, Goals, Periods, hours_label};
use gpui::*;
use gpui_kit::component::{
    button::*,
    input::{InputEvent, InputState, NumberInput},
    progress::Progress,
    *,
};

pub struct SummaryPanel {
    month: Entity<InputState>,
    week: Entity<InputState>,
    error: Option<Message>,
    pending_save: bool,
    saved: bool,
    _subscriptions: Vec<Subscription>,
}

impl SummaryPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let goals = cx.global::<State>().snapshot.goals.clone();
        let month = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(goals.month_hours.to_string())
                .step(1.)
                .min(0.25)
                .max(10000.)
        });
        let week = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(goals.week_hours.to_string())
                .step(1.)
                .min(0.25)
                .max(10000.)
        });
        let subscriptions = [&month, &week]
            .into_iter()
            .map(|input| {
                cx.subscribe_in(input, window, |this, _, event: &InputEvent, _, cx| {
                    this.saved = false;
                    if matches!(event, InputEvent::Change) {
                        this.error = None;
                    }
                    cx.notify();
                })
            })
            .collect();
        Self {
            month,
            week,
            error: None,
            pending_save: false,
            saved: false,
            _subscriptions: subscriptions,
        }
    }

    fn card(
        id: &'static str,
        title: &str,
        tracked: i64,
        goal: f64,
        color: u32,
        cx: &App,
    ) -> AnyElement {
        let language = cx.global::<State>().snapshot.language;
        let t = |key| language.text(key);
        let progress = GoalProgress::new(tracked, goal);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(title.to_owned()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(language.format(Key::Goal, &[&goal.to_string()])),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(hours_label(tracked)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(color))
                            .child(format!("{:.0} %", progress.percent)),
                    ),
            )
            .child(
                Progress::new(id)
                    .value(progress.percent)
                    .color(rgb(color))
                    .small()
                    .accessibility_label(title.to_owned()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(if progress.remaining_ms == 0 {
                        rgb(color).into()
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(if progress.remaining_ms == 0 {
                        if progress.extra_ms > 0 {
                            language.format(Key::GoalExtra, &[&hours_label(progress.extra_ms)])
                        } else {
                            t(Key::GoalReached).into()
                        }
                    } else {
                        language.format(Key::GoalRemaining, &[&hours_label(progress.remaining_ms)])
                    }),
            )
            .into_any_element()
    }
}

impl Render for SummaryPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<State>();
        let snapshot = state.snapshot.clone();
        let busy = state.busy;
        let language = snapshot.language;
        let t = |key| language.text(key);
        if self.pending_save && !busy {
            self.saved = snapshot.error.is_none();
            self.pending_save = false;
        }
        let now = chrono::Local::now();
        let mut view = div()
            .id("summary-scroll")
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t(Key::Summary)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t(Key::CurrentWorkspace)),
                    ),
            );
        if let Some(totals) = Periods::at(now)
            .ok()
            .and_then(|periods| snapshot.summary_totals(&periods, now.timestamp_millis()))
        {
            view = view
                .child(Self::card(
                    "month-progress",
                    t(Key::ThisMonth),
                    totals.month_ms,
                    snapshot.goals.month_hours,
                    0x1b9c77,
                    cx,
                ))
                .child(Self::card(
                    "week-progress",
                    t(Key::ThisWeek),
                    totals.week_ms,
                    snapshot.goals.week_hours,
                    0x5579df,
                    cx,
                ));
            if snapshot.synced_at.is_none() || snapshot.sync_error.is_some() {
                view = view.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(Key::LastVerified)),
                );
            }
        } else {
            view = view.child(
                div()
                    .py_6()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(if busy {
                        t(Key::LoadingHours)
                    } else {
                        t(Key::HoursUnavailable)
                    })
                    .child(Progress::new("summary-loading").loading(busy)),
            );
        }
        view.child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .pt_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child(t(Key::MyGoals)),
                )
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(t(Key::MonthlyHours)),
                                )
                                .child(
                                    NumberInput::new(&self.month)
                                        .small()
                                        .disabled(busy)
                                        .accessibility_label(t(Key::MonthlyGoal)),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(t(Key::WeeklyHours)),
                                )
                                .child(
                                    NumberInput::new(&self.week)
                                        .small()
                                        .disabled(busy)
                                        .accessibility_label(t(Key::WeeklyGoal)),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            Button::new("save-goals")
                                .primary()
                                .small()
                                .label(if self.saved {
                                    t(Key::Saved)
                                } else {
                                    t(Key::SaveGoals)
                                })
                                .disabled(busy)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    match Goals::parse(
                                        &this.month.read(cx).value(),
                                        &this.week.read(cx).value(),
                                    ) {
                                        Ok(goals) => {
                                            this.error = None;
                                            this.pending_save = true;
                                            send(Command::SaveGoals(goals), cx);
                                        }
                                        Err(error) => {
                                            this.error = Some(Message::from_error(error));
                                            cx.notify();
                                        }
                                    }
                                })),
                        )
                        .child(
                            Button::new("reload-summary")
                                .ghost()
                                .small()
                                .label(t(Key::Refresh))
                                .disabled(busy)
                                .on_click(|_, _, cx| send(Command::LoadSummary, cx)),
                        ),
                ),
        )
        .children(self.error.clone().map(|error| {
            div()
                .text_xs()
                .text_color(cx.theme().danger)
                .child(error.render(language))
        }))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(t(Key::SummaryHelp)),
        )
    }
}
