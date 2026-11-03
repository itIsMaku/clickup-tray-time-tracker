use crate::storage::{self, Preferences};
use anyhow::{Context, Result};
use clickup_tracker::locale::{Key, Language, Message};
use clickup_tracker::{
    api::{API_BASE, ClickUp},
    model::{DestinationList, Task, TimeEntry, User, Workspace},
    summary::{Goals, Periods, Summary},
};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub enum Command {
    Connect(String),
    SetLanguage(Language),
    SelectWorkspace(String),
    Refresh,
    Start(Task),
    Stop,
    Lookup(String),
    LoadLists,
    LoadSummary,
    SaveGoals(Goals),
    Create { list: String, name: String },
    Disconnect,
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub language: Language,
    pub user: Option<User>,
    pub workspaces: Vec<Workspace>,
    pub workspace: Option<String>,
    pub tasks: Vec<Task>,
    pub recent: Vec<Task>,
    pub current: Option<TimeEntry>,
    pub error: Option<Message>,
    pub sync_error: Option<Message>,
    pub synced_at: Option<i64>,
    pub ready: bool,
    pub lists: Vec<DestinationList>,
    pub selected_list: Option<String>,
    pub created_task: Option<Task>,
    pub summary: Option<Summary>,
    pub goals: Goals,
}

pub struct Event {
    pub snapshot: Snapshot,
    pub completed: bool,
}

impl Snapshot {
    pub fn summary_totals(
        &self,
        periods: &Periods,
        now: i64,
    ) -> Option<clickup_tracker::summary::Totals> {
        self.summary.as_ref().map(|summary| {
            summary.totals(
                if self.synced_at.is_some() {
                    self.current.as_ref()
                } else {
                    None
                },
                periods,
                now,
            )
        })
    }
}

struct Worker {
    client: Option<ClickUp>,
    preferences: Preferences,
    state: Snapshot,
}

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod http_test_support;

pub fn spawn() -> (mpsc::Sender<Command>, async_channel::Receiver<Event>) {
    let (sender, commands) = mpsc::channel();
    let (events, receiver) = async_channel::unbounded();
    thread::Builder::new()
        .name("clickup-api".into())
        .spawn(move || {
            let mut worker = Worker {
                client: None,
                preferences: Preferences::default(),
                state: Snapshot::default(),
            };
            let result = worker.restore();
            worker.state.ready = true;
            worker.report(result);
            let _ = events.send_blocking(Event {
                snapshot: worker.state.clone(),
                completed: true,
            });
            let mut last_sync = Instant::now();
            loop {
                let timeout = Duration::from_secs(30).saturating_sub(last_sync.elapsed());
                let completed = match commands.recv_timeout(timeout) {
                    Ok(command) => {
                        worker.state.error = None;
                        let mutation = matches!(
                            command,
                            Command::Start(_) | Command::Stop | Command::Create { .. }
                        );
                        let result = worker.handle(command);
                        // A timeout may have happened after the server accepted a mutation.
                        // Reconcile once; never retry a write blindly.
                        if result.is_err() && mutation {
                            let _ = worker.sync_current();
                        }
                        if mutation && worker.state.summary.is_some() {
                            worker.state.sync_error =
                                worker.load_summary_entries().err().map(Message::from_error);
                        }
                        worker.report(result);
                        true
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if worker.client.is_some() {
                            let result = if worker.state.summary.is_some() {
                                worker.load_summary_entries()
                            } else {
                                worker.sync_current()
                            };
                            worker.state.sync_error = result.err().map(Message::from_error);
                        }
                        false
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                last_sync = Instant::now();
                if events
                    .send_blocking(Event {
                        snapshot: worker.state.clone(),
                        completed,
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .expect("Cannot create ClickUp worker");
    (sender, receiver)
}

impl Worker {
    fn disconnect_with(
        &mut self,
        forget_token: impl FnOnce() -> Result<()>,
        save: impl FnOnce(&Preferences) -> Result<()>,
    ) -> Result<()> {
        forget_token()?;
        self.preferences = Preferences {
            language: self.state.language,
            ..Preferences::default()
        };
        self.client = None;
        self.state = Snapshot {
            language: self.preferences.language,
            ready: true,
            ..Snapshot::default()
        };
        save(&self.preferences).context(Message::new(Key::SavePreferences))
    }
    fn report(&mut self, result: Result<()>) {
        if let Err(error) = result {
            self.state.error = Some(Message::from_error(error));
        }
    }

    fn restore(&mut self) -> Result<()> {
        self.preferences = storage::load()?;
        self.state.language = self.preferences.language;
        if let Some(token) = storage::token()? {
            self.connect(&token, false)?;
        }
        Ok(())
    }

    fn connect(&mut self, token: &str, save: bool) -> Result<()> {
        let client = ClickUp::new(token, API_BASE)?;
        let user = client.user()?;
        let workspaces = client.workspaces()?;
        anyhow::ensure!(!workspaces.is_empty(), Message::new(Key::NoWorkspaces));
        if save {
            storage::set_token(token)?;
        }
        let workspace = self
            .preferences
            .workspace
            .as_ref()
            .filter(|id| workspaces.iter().any(|w| &w.id == *id))
            .cloned()
            .unwrap_or_else(|| workspaces[0].id.clone());
        self.client = Some(client);
        self.state.user = Some(user);
        self.state.workspaces = workspaces;
        self.select_workspace(workspace)
    }

    fn api(&self) -> Result<&ClickUp> {
        self.client
            .as_ref()
            .context(Message::new(Key::ConnectFirst))
    }
    fn workspace(&self) -> Result<&str> {
        self.state
            .workspace
            .as_deref()
            .context(Message::new(Key::ChooseWorkspace))
    }

    fn persist(&self) -> Result<()> {
        storage::save(&self.preferences).context(Message::new(Key::SavePreferences))
    }

    fn reset_workspace(&mut self, workspace: String) -> Result<()> {
        anyhow::ensure!(
            self.state.workspaces.iter().any(|w| w.id == workspace),
            Message::new(Key::UnknownWorkspace)
        );
        self.state.workspace = Some(workspace.clone());
        self.preferences.workspace = Some(workspace.clone());
        self.state.tasks.clear();
        self.state.current = None;
        self.state.synced_at = None;
        self.state.sync_error = None;
        self.state.lists.clear();
        self.state.selected_list = self.preferences.last_list.get(&workspace).cloned();
        self.state.created_task = None;
        self.state.summary = None;
        self.state.goals = self
            .preferences
            .goals
            .get(&workspace)
            .cloned()
            .unwrap_or_default();
        self.state.recent = self
            .preferences
            .recent
            .get(&workspace)
            .cloned()
            .unwrap_or_default();
        Ok(())
    }

    fn select_workspace(&mut self, workspace: String) -> Result<()> {
        self.reset_workspace(workspace)?;
        let saved = self.persist();
        self.refresh()?;
        saved
    }

    fn sync_current(&mut self) -> Result<()> {
        self.state.synced_at = None;
        self.state.current = self.api()?.current(self.workspace()?)?;
        self.state.synced_at = Some(chrono::Utc::now().timestamp_millis());
        self.state.sync_error = None;
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        if self.state.summary.is_some() {
            self.load_summary_entries()?;
        } else {
            self.sync_current()?;
        }
        let user = self
            .state
            .user
            .as_ref()
            .context(Message::new(Key::MissingUser))?
            .id;
        self.state.tasks = self.api()?.tasks(self.workspace()?, user)?;
        self.state.tasks.sort_by_key(|t| t.name.to_lowercase());
        Ok(())
    }

    fn load_summary_entries(&mut self) -> Result<()> {
        let now = chrono::Local::now();
        let periods = Periods::at(now)?;
        let entries = self.api()?.time_entries(
            self.workspace()?,
            periods.month_start.min(periods.week_start),
            now.timestamp_millis(),
        )?;
        let fetched_at = chrono::Utc::now().timestamp_millis();
        self.state.summary = Some(Summary {
            entries,
            fetched_at,
        });
        self.sync_current()
    }

    fn remember(&mut self, task: Task) -> Result<()> {
        let workspace = self.workspace()?.to_owned();
        let recent = self.preferences.recent.entry(workspace).or_default();
        recent.retain(|t| t.id != task.id);
        recent.insert(0, task);
        recent.truncate(8);
        self.state.recent = recent.clone();
        self.persist()
    }

    fn handle(&mut self, command: Command) -> Result<()> {
        match command {
            Command::SetLanguage(language) => {
                self.preferences.language = language;
                self.state.language = language;
                self.persist()
            }
            Command::Connect(token) => self.connect(&token, true),
            Command::SelectWorkspace(id) => self.select_workspace(id),
            Command::Refresh => self.refresh(),
            Command::Start(task) => {
                self.state.synced_at = None;
                let entry = self.api()?.switch_to(self.workspace()?, &task.id)?;
                self.state.current = Some(entry);
                self.state.synced_at = Some(chrono::Utc::now().timestamp_millis());
                self.remember(task)
            }
            Command::Stop => {
                self.sync_current()?;
                if self.state.current.is_some() {
                    self.state.synced_at = None;
                    self.api()?.stop(self.workspace()?)?;
                }
                self.sync_current()
            }
            Command::Lookup(input) => {
                let task = self.api()?.task_in_workspace(&input, self.workspace()?)?;
                // This adds the task to the picker; it does not start tracking.
                self.remember(task)
            }
            Command::LoadLists => {
                self.state.created_task = None;
                self.state.lists = self.api()?.lists(self.workspace()?)?;
                if !self
                    .state
                    .lists
                    .iter()
                    .any(|l| Some(&l.id) == self.state.selected_list.as_ref())
                {
                    self.state.selected_list = self.state.lists.first().map(|l| l.id.clone());
                }
                Ok(())
            }
            Command::LoadSummary => self.load_summary_entries(),
            Command::SaveGoals(goals) => {
                let checked = Goals::parse(
                    &goals.month_hours.to_string(),
                    &goals.week_hours.to_string(),
                )?;
                let workspace = self.workspace()?.to_owned();
                let mut preferences = self.preferences.clone();
                preferences.goals.insert(workspace, checked.clone());
                storage::save(&preferences).context(Message::new(Key::SaveGoalsError))?;
                self.preferences = preferences;
                self.state.goals = checked;
                Ok(())
            }
            Command::Create { list, name } => {
                anyhow::ensure!(
                    self.state.lists.iter().any(|l| l.id == list),
                    Message::new(Key::ChooseWorkspaceList)
                );
                let user = self
                    .state
                    .user
                    .as_ref()
                    .context(Message::new(Key::MissingUser))?
                    .id;
                let task = self.api()?.create_task(&list, &name, user)?;
                // Preserve the created task even if starting the timer fails.
                // The user can retry tracking from recents without creating a duplicate.
                self.state.created_task = Some(task.clone());
                self.state.tasks.insert(0, task.clone());
                self.state.selected_list = Some(list.clone());
                self.preferences
                    .last_list
                    .insert(self.workspace()?.to_owned(), list);
                self.remember(task.clone())
                    .context(Message::new(Key::CreatedPreferencesError))?;
                self.state.synced_at = None;
                self.state.current = Some(
                    self.api()?
                        .switch_to(self.workspace()?, &task.id)
                        .context(Message::new(Key::CreatedTrackingError))?,
                );
                self.state.synced_at = Some(chrono::Utc::now().timestamp_millis());
                Ok(())
            }
            Command::Disconnect => self.disconnect_with(storage::forget_token, storage::save),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::http_test_support::server;
    use super::*;

    fn connected_worker(base: &str) -> Worker {
        Worker {
            client: Some(ClickUp::new("pk_test", base).unwrap()),
            preferences: Preferences::default(),
            state: Snapshot {
                language: Language::Czech,
                user: Some(User {
                    id: 7,
                    username: "Example User".into(),
                }),
                workspace: Some("42".into()),
                workspaces: vec![Workspace {
                    id: "42".into(),
                    name: "Example".into(),
                }],
                ready: true,
                ..Snapshot::default()
            },
        }
    }

    #[test]
    fn disconnect_clears_authenticated_memory_even_when_preferences_cannot_be_saved() {
        let mut worker = connected_worker("http://127.0.0.1:1");
        let deleted = std::cell::Cell::new(false);
        let result = worker.disconnect_with(
            || {
                deleted.set(true);
                Ok(())
            },
            |_| anyhow::bail!("Disk unavailable"),
        );
        assert!(result.is_err());
        assert!(deleted.get());
        assert!(worker.client.is_none());
        assert!(worker.state.user.is_none());
        assert!(worker.state.current.is_none());
        assert_eq!(worker.state.language, Language::Czech);
    }

    #[test]
    fn successful_manual_refresh_clears_the_previous_sync_error() {
        let (base, handle) = server(vec![
            ("GET /api/v2/team/42/time_entries/current ", 500, "{}"),
            (
                "GET /api/v2/team/42/time_entries/current ",
                200,
                r#"{"data":null}"#,
            ),
            ("GET /api/v2/team/42/task?", 200, r#"{"tasks":[]}"#),
        ]);
        let mut worker = connected_worker(&base);
        worker.state.sync_error = Some(Message::from_error(worker.sync_current().unwrap_err()));
        worker.handle(Command::Refresh).unwrap();
        assert!(worker.state.sync_error.is_none());
        assert!(worker.state.synced_at.is_some());
        handle.join().unwrap();
    }

    #[test]
    fn changing_workspace_discards_the_previous_workspace_sync_error() {
        let mut worker = connected_worker("http://127.0.0.1:1");
        worker.state.sync_error = Some(Message::new(Key::Unavailable));
        worker.state.workspaces.push(Workspace {
            id: "99".into(),
            name: "Second workspace".into(),
        });
        worker.reset_workspace("99".into()).unwrap();
        assert!(worker.state.sync_error.is_none());
        assert_eq!(worker.state.workspace.as_deref(), Some("99"));
        assert!(worker.state.current.is_none());
    }

    #[test]
    fn fetched_history_remains_available_when_current_verification_fails() {
        let (base, handle) = server(vec![
            (
                "GET /api/v2/team/42/time_entries?start_date=",
                200,
                r#"{"data":[{"id":"complete","start":"1000","duration":3600000}]}"#,
            ),
            ("GET /api/v2/team/42/time_entries/current ", 500, "{}"),
        ]);
        let mut worker = connected_worker(&base);
        assert!(worker.handle(Command::LoadSummary).is_err());
        assert!(worker.state.synced_at.is_none());
        let totals = worker
            .state
            .summary_totals(
                &Periods {
                    month_start: 0,
                    week_start: 0,
                },
                10000000,
            )
            .expect("Completed history must be visible");
        assert_eq!(totals.month_ms, 3600000);
        handle.join().unwrap();
    }
    #[test]
    fn unverified_current_does_not_inflate_summary_during_an_outage() {
        let entry: TimeEntry =
            serde_json::from_str(r#"{"id":"a","start":"2000","duration":-2000}"#).unwrap();
        let state = Snapshot {
            summary: Some(Summary {
                entries: vec![entry.clone()],
                fetched_at: 5000,
            }),
            current: Some(entry),
            synced_at: None,
            ..Snapshot::default()
        };
        assert_eq!(
            state
                .summary_totals(
                    &Periods {
                        month_start: 0,
                        week_start: 0
                    },
                    9000
                )
                .unwrap()
                .month_ms,
            3000
        );
    }
}
