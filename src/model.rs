use crate::locale::{Key, Message};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub list: Option<TaskList>,
    #[serde(default)]
    pub team_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaskList {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DestinationList {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub location: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    #[serde(default)]
    pub wid: Option<String>,
    #[serde(default)]
    pub task: Option<Task>,
    pub start: String,
    #[serde(deserialize_with = "string_or_integer")]
    pub duration: String,
}

fn string_or_integer<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Text(String),
        Number(i64),
    }
    Ok(match Value::deserialize(deserializer)? {
        Value::Text(value) => value,
        Value::Number(value) => value.to_string(),
    })
}

impl TimeEntry {
    pub fn elapsed_ms(&self, now: i64) -> i64 {
        let duration = self.duration.parse::<i64>().unwrap_or(0);
        if duration < 0 {
            now.saturating_sub(self.start.parse().unwrap_or(now)).max(0)
        } else {
            duration
        }
    }
}

pub fn task_id(input: &str) -> anyhow::Result<String> {
    let input = input.trim();
    let id = if input.starts_with("https://") {
        let url = reqwest::Url::parse(input)?;
        anyhow::ensure!(
            url.host_str() == Some("app.clickup.com"),
            Message::new(Key::TaskLinkHost)
        );
        let parts: Vec<_> = url.path_segments().into_iter().flatten().collect();
        let index = parts
            .iter()
            .position(|s| *s == "t")
            .ok_or_else(|| anyhow::anyhow!(Message::new(Key::TaskLinkPath)))?;
        parts.get(index + 1).copied().unwrap_or("").to_owned()
    } else {
        input.to_owned()
    };
    anyhow::ensure!(
        !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric()),
        Message::new(Key::TaskId)
    );
    Ok(id)
}

pub fn format_time(millis: i64) -> String {
    let seconds = millis.max(0) / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
