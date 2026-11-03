use crate::locale::{Key, Message};
use crate::model::{DestinationList, Task, TimeEntry, User, Workspace};
use anyhow::Result;
use reqwest::{Method, blocking::Client};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

pub const API_BASE: &str = "https://api.clickup.com/api/v2";

pub struct ClickUp {
    http: Client,
    token: String,
    base: String,
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

impl ClickUp {
    pub fn time_entries(&self, workspace: &str, start: i64, end: i64) -> Result<Vec<TimeEntry>> {
        Ok(self
            .request::<Data<Vec<TimeEntry>>>(
                Method::GET,
                &format!("/team/{workspace}/time_entries?start_date={start}&end_date={end}"),
                None,
            )?
            .data)
    }
    pub fn lists(&self, workspace: &str) -> Result<Vec<DestinationList>> {
        #[derive(Deserialize)]
        struct Space {
            id: String,
            name: String,
        }
        #[derive(Deserialize)]
        struct Spaces {
            spaces: Vec<Space>,
        }
        #[derive(Deserialize)]
        struct Lists {
            lists: Vec<DestinationList>,
        }
        #[derive(Deserialize)]
        struct Folder {
            name: String,
            #[serde(default)]
            lists: Vec<DestinationList>,
        }
        #[derive(Deserialize)]
        struct Folders {
            folders: Vec<Folder>,
        }
        let spaces: Spaces = self.request(
            Method::GET,
            &format!("/team/{workspace}/space?archived=false"),
            None,
        )?;
        let mut lists = Vec::new();
        for space in spaces.spaces {
            let folderless: Lists = self.request(
                Method::GET,
                &format!("/space/{}/list?archived=false", space.id),
                None,
            )?;
            for mut list in folderless.lists {
                list.location = space.name.clone();
                lists.push(list);
            }
            let folders: Folders = self.request(
                Method::GET,
                &format!("/space/{}/folder?archived=false", space.id),
                None,
            )?;
            for folder in folders.folders {
                for mut list in folder.lists {
                    list.location = format!("{} / {}", space.name, folder.name);
                    lists.push(list);
                }
            }
        }
        lists.sort_by_key(|l| format!("{}/{}", l.location, l.name).to_lowercase());
        Ok(lists)
    }

    pub fn create_task(&self, list: &str, name: &str, user: u64) -> Result<Task> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), Message::new(Key::EnterTaskName));
        anyhow::ensure!(
            !list.is_empty() && list.bytes().all(|b| b.is_ascii_digit()),
            Message::new(Key::ValidList)
        );
        self.request(
            Method::POST,
            &format!("/list/{list}/task"),
            Some(serde_json::json!({"name":name,"assignees":[user]})),
        )
        .map_err(|error| error.context(Message::new(Key::CreateUnconfirmed)))
    }
    pub fn new(token: &str, base: &str) -> Result<Self> {
        anyhow::ensure!(!token.trim().is_empty(), Message::new(Key::EnterToken));
        Ok(Self {
            http: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("ClickUpTracker/0.1.0")
                .build()?,
            token: token.trim().to_owned(),
            base: base.trim_end_matches('/').to_owned(),
        })
    }

    fn send_request(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::blocking::Response> {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header("Authorization", &self.token)
            .header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .map_err(|_| anyhow::anyhow!(Message::new(Key::Unavailable)))?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!(match status.as_u16() {
                401 => Message::new(Key::InvalidToken).to_owned(),
                403 => Message::new(Key::Forbidden).to_owned(),
                429 => Message::new(Key::RateLimited).to_owned(),
                _ => Message::with_value(Key::HttpError, status.as_u16()),
            });
        }
        Ok(response)
    }

    fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        self.send_request(method, path, body)?
            .json()
            .map_err(|_| anyhow::anyhow!(Message::new(Key::UnexpectedResponse)))
    }

    pub fn user(&self) -> Result<User> {
        #[derive(Deserialize)]
        struct Response {
            user: User,
        }
        Ok(self.request::<Response>(Method::GET, "/user", None)?.user)
    }

    pub fn workspaces(&self) -> Result<Vec<Workspace>> {
        #[derive(Deserialize)]
        struct Response {
            teams: Vec<Workspace>,
        }
        Ok(self.request::<Response>(Method::GET, "/team", None)?.teams)
    }

    pub fn tasks(&self, workspace: &str, user: u64) -> Result<Vec<Task>> {
        #[derive(Deserialize)]
        struct Response {
            tasks: Vec<Task>,
            #[serde(default)]
            last_page: bool,
        }
        let mut tasks = Vec::new();
        for page in 0.. {
            let path = format!(
                "/team/{workspace}/task?page={page}&subtasks=true&include_closed=false&assignees%5B%5D={user}"
            );
            let response: Response = self.request(Method::GET, &path, None)?;
            let done = response.last_page || response.tasks.len() < 100;
            tasks.extend(response.tasks);
            if done {
                return Ok(tasks);
            }
            // Stay within ClickUp's lowest-tier rate limit during large workspaces.
            std::thread::sleep(Duration::from_millis(650));
        }
        unreachable!()
    }

    pub fn task(&self, id: &str) -> Result<Task> {
        let id = crate::model::task_id(id)?;
        self.request(Method::GET, &format!("/task/{id}"), None)
    }

    pub fn task_in_workspace(&self, id: &str, workspace: &str) -> Result<Task> {
        let task = self.task(id)?;
        anyhow::ensure!(
            task.team_id.as_deref() == Some(workspace),
            Message::new(Key::TaskWorkspace)
        );
        Ok(task)
    }

    pub fn current(&self, workspace: &str) -> Result<Option<TimeEntry>> {
        let current = self
            .request::<Data<Option<TimeEntry>>>(
                Method::GET,
                &format!("/team/{workspace}/time_entries/current"),
                None,
            )?
            .data;
        if let Some(entry) = &current {
            anyhow::ensure!(
                entry.wid.as_deref().is_none_or(|id| id == workspace),
                Message::new(Key::TimerWorkspace)
            );
        }
        Ok(current)
    }

    pub fn stop(&self, workspace: &str) -> Result<()> {
        self.send_request(
            Method::POST,
            &format!("/team/{workspace}/time_entries/stop"),
            Some(serde_json::json!({})),
        )?;
        Ok(())
    }

    pub fn switch_to(&self, workspace: &str, task: &str) -> Result<TimeEntry> {
        let task = crate::model::task_id(task)?;
        let current = self.current(workspace)?;
        if let Some(current) = &current
            && current.task.as_ref().is_some_and(|t| t.id == task)
        {
            return Ok(current.clone());
        }
        // Validate access and workspace before interrupting any existing timer.
        self.task_in_workspace(&task, workspace)?;
        if current.is_some() {
            self.stop(workspace)?;
        }
        // A write can succeed even if its response body differs from /current,
        // or the response is lost. Confirm the intended state without retrying it.
        let write = self
            .send_request(
                Method::POST,
                &format!("/team/{workspace}/time_entries/start"),
                Some(serde_json::json!({"tid": task})),
            )
            .map(|_| ());
        match self.current(workspace) {
            Ok(Some(entry)) if entry.task.as_ref().is_some_and(|t| t.id == task) => Ok(entry),
            Ok(_) => {
                write?;
                anyhow::bail!(Message::new(Key::StartUnconfirmed));
            }
            Err(error) => {
                write?;
                Err(error.context(Message::new(Key::StartUnverified)))
            }
        }
    }
}
