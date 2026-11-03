use crate::{State, send, worker::Command};
use clickup_tracker::locale::{Key, Language};
use clickup_tracker::model::{Task as ClickUpTask, format_time};
use gpui::prelude::*;
use gpui::*;
use gpui_kit::component::{
    button::*,
    input::{Input, InputEvent, InputState},
    select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    *,
};
use std::{collections::HashSet, time::Duration};

gpui_kit::actions!(tracker, [Dismiss, Quit]);

#[derive(Clone)]
struct Choice {
    id: String,
    label: String,
}
impl SelectItem for Choice {
    type Value = String;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &String {
        &self.id
    }
}
type ChoiceSelect = SelectState<SearchableVec<Choice>>;

pub struct Tracker {
    token: Entity<InputState>,
    search: Entity<InputState>,
    task_name: Entity<InputState>,
    focus: FocusHandle,
    settings: bool,
    creating: bool,
    selected_list: Option<String>,
    workspace_select: Entity<ChoiceSelect>,
    list_select: Entity<ChoiceSelect>,
    workspace_choices: Vec<(String, String)>,
    list_choices: Vec<(String, String)>,
    summary_panel: Option<Entity<crate::summary_ui::SummaryPanel>>,
    pending_create: bool,
    was_active: bool,
    language: Language,
    _subscriptions: Vec<Subscription>,
}

impl Tracker {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = cx.global::<State>().snapshot.language;
        let t = move |key| language.text(key);
        let token = cx.new(|cx| InputState::new(window, cx).placeholder("pk_…").masked(true));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(t(Key::Search)));
        let task_name = cx.new(|cx| InputState::new(window, cx).placeholder(t(Key::TaskName)));
        let workspace_select = cx.new(|cx| {
            SelectState::new(SearchableVec::<Choice>::new(Vec::new()), None, window, cx)
                .searchable(true)
        });
        let list_select = cx.new(|cx| {
            SelectState::new(SearchableVec::<Choice>::new(Vec::new()), None, window, cx)
                .searchable(true)
        });
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let subscriptions = vec![
            cx.subscribe_in(
                &workspace_select,
                window,
                |this, _, event: &SelectEvent<SearchableVec<Choice>>, _, cx| {
                    if let SelectEvent::Confirm(Some(id)) = event
                        && cx.global::<State>().snapshot.workspace.as_ref() != Some(id)
                    {
                        this.summary_panel = None;
                        send(Command::SelectWorkspace(id.clone()), cx);
                    }
                },
            ),
            cx.subscribe_in(
                &list_select,
                window,
                |this, _, event: &SelectEvent<SearchableVec<Choice>>, _, cx| {
                    let SelectEvent::Confirm(id) = event;
                    this.selected_list = id.clone();
                    cx.notify();
                },
            ),
            cx.subscribe_in(&search, window, |_, _, _: &InputEvent, _, cx| cx.notify()),
            cx.subscribe_in(
                &task_name,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.submit_task(window, cx);
                    }
                    cx.notify();
                },
            ),
            cx.subscribe_in(&token, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.connect(window, cx);
                }
            }),
            cx.observe_window_activation(window, |this, window, _| {
                if window.is_window_active() {
                    this.was_active = true;
                } else if this.was_active {
                    window.remove_window();
                }
            }),
        ];
        // Only a visible popup has a repaint timer. Closing it releases this entity.
        cx.spawn(async move |entity, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if entity
                    .update(cx, |_, cx| {
                        if cx.global::<State>().snapshot.current.is_some() {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            token,
            search,
            task_name,
            focus,
            settings: false,
            creating: false,
            selected_list: None,
            workspace_select,
            list_select,
            workspace_choices: Vec::new(),
            list_choices: Vec::new(),
            summary_panel: None,
            pending_create: false,
            was_active: window.is_window_active(),
            language,
            _subscriptions: subscriptions,
        }
    }

    fn submit_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = cx.global::<State>();
        if !self.creating || state.busy {
            return;
        }
        let Some(list) = self
            .selected_list
            .clone()
            .or_else(|| state.snapshot.selected_list.clone())
        else {
            return;
        };
        let name = self.task_name.read(cx).value().to_string();
        if name.trim().is_empty() {
            return;
        }
        self.pending_create = true;
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        send(Command::Create { list, name }, cx);
    }

    fn connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let token = self.token.read(cx).value().to_string();
        if token.trim().is_empty() || cx.global::<State>().busy {
            return;
        }
        self.token
            .update(cx, |input, cx| input.set_value("", window, cx));
        send(Command::Connect(token), cx);
    }

    fn task_row(&self, task: ClickUpTask, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let running = cx
            .global::<State>()
            .snapshot
            .current
            .as_ref()
            .and_then(|e| e.task.as_ref())
            .is_some_and(|t| t.id == task.id);
        let subtitle = task
            .list
            .as_ref()
            .map(|l| l.name.clone())
            .unwrap_or_else(|| format!("#{}", task.id));
        let name = task.name.clone();
        div()
            .id(("task", index))
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(cx.theme().secondary))
            .on_click(move |_, _, cx| send(Command::Start(task.clone()), cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().truncate().child(name))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate()
                            .child(subtitle),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if running {
                        rgb(0x22a878).into()
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(if running { "●" } else { "▶" }),
            )
            .into_any_element()
    }

    fn section(title: impl Into<SharedString>, cx: &App) -> AnyElement {
        div()
            .px_3()
            .pt_3()
            .pb_1()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(title.into())
            .into_any_element()
    }
}

impl Render for Tracker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<State>();
        let snapshot = state.snapshot.clone();
        let busy = state.busy;
        let language = snapshot.language;
        let t = move |key| language.text(key);
        if self.language != language {
            self.language = language;
            self.search.update(cx, |input, cx| {
                input.set_placeholder(t(Key::Search), window, cx)
            });
            self.task_name.update(cx, |input, cx| {
                input.set_placeholder(t(Key::TaskName), window, cx)
            });
        }
        let connected = snapshot.user.is_some();
        let workspace_choices: Vec<_> = snapshot
            .workspaces
            .iter()
            .map(|w| (w.id.clone(), w.name.clone()))
            .collect();
        if self.workspace_choices != workspace_choices {
            self.workspace_choices = workspace_choices.clone();
            self.workspace_select.update(cx, |state, cx| {
                state.set_items(
                    SearchableVec::new(
                        workspace_choices
                            .into_iter()
                            .map(|(id, label)| Choice { id, label })
                            .collect::<Vec<_>>(),
                    ),
                    window,
                    cx,
                )
            });
        }
        if let Some(id) = &snapshot.workspace
            && self.workspace_select.read(cx).selected_value() != Some(id)
        {
            self.workspace_select
                .update(cx, |state, cx| state.set_selected_value(id, window, cx));
        }
        let list_choices: Vec<_> = snapshot
            .lists
            .iter()
            .map(|l| (l.id.clone(), format!("{} / {}", l.location, l.name)))
            .collect();
        if self.list_choices != list_choices {
            self.list_choices = list_choices.clone();
            self.list_select.update(cx, |state, cx| {
                state.set_items(
                    SearchableVec::new(
                        list_choices
                            .into_iter()
                            .map(|(id, label)| Choice { id, label })
                            .collect::<Vec<_>>(),
                    ),
                    window,
                    cx,
                );
                if let Some(id) = &snapshot.selected_list {
                    state.set_selected_value(id, window, cx);
                }
            });
        }
        if self.pending_create && !busy {
            if snapshot.created_task.is_some() {
                self.creating = false;
            }
            self.pending_create = false;
        }
        let mut body = div().flex().flex_col().gap_3().flex_1().min_h_0();

        if !snapshot.ready {
            body = body
                .justify_center()
                .items_center()
                .child(t(Key::ConnectingClickUp));
        } else if !connected && !self.settings {
            body = body
                .justify_center()
                .gap_4()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t(Key::Welcome)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(Key::TokenHelp)),
                )
                .child(Input::new(&self.token))
                .child(
                    Button::new("connect")
                        .primary()
                        .label(if busy {
                            t(Key::Connecting)
                        } else {
                            t(Key::Connect)
                        })
                        .disabled(busy)
                        .on_click(cx.listener(|this, _, window, cx| this.connect(window, cx))),
                )
                .child(
                    Button::new("get-token")
                        .ghost()
                        .label(t(Key::FindToken))
                        .on_click(|_, _, cx| cx.open_url("https://app.clickup.com/settings/apps")),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(Key::TokenPrivacy)),
                );
        } else if self.settings {
            body = body
                .gap_4()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t(Key::Settings)),
                )
                .when_some(snapshot.user.as_ref(), |body, user| {
                    body.child(language.format(Key::ConnectedUser, &[&user.username]))
                })
                .child(div().text_sm().child(t(Key::Language)))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("language-en")
                                .label(t(Key::English))
                                .disabled(busy || language == Language::English)
                                .on_click(|_, _, cx| {
                                    send(Command::SetLanguage(Language::English), cx)
                                }),
                        )
                        .child(
                            Button::new("language-cs")
                                .label(t(Key::Czech))
                                .disabled(busy || language == Language::Czech)
                                .on_click(|_, _, cx| {
                                    send(Command::SetLanguage(Language::Czech), cx)
                                }),
                        ),
                )
                .child(
                    Button::new("disconnect")
                        .danger()
                        .label(t(Key::Disconnect))
                        .disabled(busy || !connected)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.settings = false;
                            send(Command::Disconnect, cx);
                        })),
                )
                .child(
                    Button::new("back")
                        .label(t(Key::Back))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.settings = false;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("quit")
                        .ghost()
                        .label(t(Key::QuitApp))
                        .on_click(|_, _, cx| cx.quit()),
                );
        } else if self.creating {
            let selected = self
                .selected_list
                .as_ref()
                .or(snapshot.selected_list.as_ref());
            body = body
                .gap_4()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t(Key::NewTask)),
                )
                .child(Input::new(&self.task_name))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(Key::ListHeading)),
                )
                .child(
                    Select::new(&self.list_select)
                        .search_placeholder(t(Key::SearchPlaceholder))
                        .empty(move |_, _| div().p_3().child(t(Key::NoResults)))
                        .placeholder(t(Key::SelectList))
                        .menu_max_h(px(260.))
                        .disabled(busy)
                        .w_full(),
                )
                .child(div().flex_1());
            body = body
                .when(snapshot.lists.is_empty(), |body| {
                    body.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(if busy {
                                t(Key::LoadingLists)
                            } else {
                                t(Key::NoLists)
                            }),
                    )
                })
                .child(
                    Button::new("create-task")
                        .primary()
                        .label(if busy {
                            t(Key::Working)
                        } else {
                            t(Key::CreateTrack)
                        })
                        .disabled(
                            busy || selected.is_none()
                                || self.task_name.read(cx).value().trim().is_empty(),
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.submit_task(window, cx))),
                )
                .child(
                    Button::new("cancel-create")
                        .ghost()
                        .label(t(Key::Back))
                        .disabled(busy)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.creating = false;
                            cx.notify();
                        })),
                );
        } else if let Some(panel) = &self.summary_panel {
            body = body.child(panel.clone());
        } else {
            let (title, elapsed, running) = match &snapshot.current {
                Some(entry) => (
                    entry
                        .task
                        .as_ref()
                        .map(|t| t.name.clone())
                        .unwrap_or_else(|| t(Key::UntitledTimer).into()),
                    format_time(entry.elapsed_ms(chrono::Utc::now().timestamp_millis())),
                    true,
                ),
                None => (t(Key::Ready).into(), "00:00:00".into(), false),
            };
            body = body
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded_lg()
                        .bg(cx.theme().secondary)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(if snapshot.synced_at.is_none() {
                                    t(Key::Unverified)
                                } else if running {
                                    t(Key::Tracking)
                                } else {
                                    t(Key::Stopped)
                                }),
                        )
                        .child(div().text_sm().truncate().child(title))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(div().text_3xl().font_family("monospace").child(elapsed))
                                .when(running, |row| {
                                    row.child(
                                        Button::new("stop")
                                            .danger()
                                            .label(t(Key::Stop))
                                            .disabled(busy)
                                            .on_click(|_, _, cx| send(Command::Stop, cx)),
                                    )
                                }),
                        ),
                )
                .child(Input::new(&self.search));

            let query = self
                .search
                .read(cx)
                .value()
                .to_string()
                .trim()
                .to_lowercase();
            let mut rows = Vec::new();
            let mut seen = HashSet::new();
            if query.is_empty() && !snapshot.recent.is_empty() {
                rows.push(Self::section(t(Key::Recent), cx));
                for task in snapshot.recent.iter().cloned() {
                    seen.insert(task.id.clone());
                    rows.push(self.task_row(task, rows.len(), cx));
                }
            }
            rows.push(Self::section(
                if query.is_empty() {
                    t(Key::MyTasks)
                } else {
                    t(Key::Results)
                },
                cx,
            ));
            let mut matches = 0;
            for task in snapshot
                .tasks
                .iter()
                .chain(snapshot.recent.iter())
                .filter(|task| {
                    task.name.to_lowercase().contains(&query)
                        || task.id.to_lowercase().contains(&query)
                        || task
                            .list
                            .as_ref()
                            .is_some_and(|l| l.name.to_lowercase().contains(&query))
                })
            {
                if seen.insert(task.id.clone()) {
                    rows.push(self.task_row(task.clone(), rows.len(), cx));
                    matches += 1;
                }
            }
            if matches == 0 && (snapshot.recent.is_empty() || !query.is_empty()) {
                rows.push(
                    div()
                        .p_3()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(if busy {
                            t(Key::LoadingTasks)
                        } else {
                            t(Key::NoTasks)
                        })
                        .into_any_element(),
                );
            }
            body = body
                .child(
                    div()
                        .id("task-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .children(rows),
                )
                .child(
                    Button::new("lookup")
                        .ghost()
                        .label(t(Key::Lookup))
                        .disabled(busy || query.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| {
                            let value = this.search.read(cx).value().to_string();
                            this.search
                                .update(cx, |input, cx| input.set_value("", window, cx));
                            send(Command::Lookup(value), cx);
                        })),
                );
        }

        let header_left = if connected && !self.settings && !self.creating {
            Select::new(&self.workspace_select)
                .search_placeholder(t(Key::SearchPlaceholder))
                .empty(move |_, _| div().p_3().child(t(Key::NoResults)))
                .placeholder(t(Key::Workspace))
                .small()
                .menu_width(px(270.))
                .menu_max_h(px(250.))
                .disabled(busy || snapshot.current.is_some())
                .w(px(180.))
                .into_any_element()
        } else {
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("ClickUp")
                .into_any_element()
        };
        div()
            .key_context("Tracker")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .text_sm()
            .on_action(|_: &Dismiss, window, _| window.remove_window())
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(header_left)
                    .child(div().flex().items_center().gap_1().when(
                        connected && !self.creating && !self.settings,
                        |row| {
                            row.child(
                                Button::new("new-task")
                                    .ghost()
                                    .small()
                                    .label(t(Key::AddTask))
                                    .disabled(busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.creating = true;
                                        this.summary_panel = None;
                                        this.selected_list = None;
                                        if let Some(id) =
                                            cx.global::<State>().snapshot.selected_list.clone()
                                        {
                                            this.list_select.update(cx, |state, cx| {
                                                state.set_selected_value(&id, window, cx)
                                            });
                                        }
                                        this.task_name.update(cx, |input, cx| {
                                            input.set_value("", window, cx)
                                        });
                                        send(Command::LoadLists, cx);
                                    })),
                            )
                        },
                    )),
            )
            .children(snapshot.error.or(snapshot.sync_error).map(|error| {
                div()
                    .rounded_md()
                    .p_3()
                    .bg(cx.theme().secondary)
                    .text_xs()
                    .text_color(cx.theme().danger)
                    .child(error.render(language))
            }))
            .child(body)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if busy {
                                "..."
                            } else if connected {
                                t(Key::Connected)
                            } else {
                                ""
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .when(connected, |row| {
                                row.child(
                                    Button::new("refresh")
                                        .ghost()
                                        .label("↻")
                                        .tooltip(t(Key::Refresh))
                                        .disabled(busy)
                                        .on_click(|_, _, cx| send(Command::Refresh, cx)),
                                )
                                .child(
                                    Button::new("summary")
                                        .ghost()
                                        .label(t(Key::Summary))
                                        .disabled(busy)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.settings = false;
                                            this.creating = false;
                                            if this.summary_panel.is_some() {
                                                this.summary_panel = None;
                                                cx.notify();
                                            } else {
                                                this.summary_panel = Some(cx.new(|cx| {
                                                    crate::summary_ui::SummaryPanel::new(window, cx)
                                                }));
                                                send(Command::LoadSummary, cx);
                                            }
                                        })),
                                )
                                .child(
                                    Button::new("settings")
                                        .ghost()
                                        .label(t(Key::Settings))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.summary_panel = None;
                                            this.creating = false;
                                            this.settings = !this.settings;
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(!connected, |row| {
                                row.child(
                                    Button::new("offline-settings")
                                        .ghost()
                                        .label(t(Key::Settings))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.settings = !this.settings;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("exit")
                                        .ghost()
                                        .label(t(Key::Quit))
                                        .on_click(|_, _, cx| cx.quit()),
                                )
                            }),
                    ),
            )
    }
}
