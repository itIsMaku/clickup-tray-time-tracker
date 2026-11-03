use anyhow::{Context, Result};
use clickup_tracker::locale::{Key, Language, Message};
use clickup_tracker::model::Task;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Preferences {
    #[serde(default)]
    pub language: Language,
    pub workspace: Option<String>,
    #[serde(default)]
    pub recent: HashMap<String, Vec<Task>>,
    #[serde(default)]
    pub last_list: HashMap<String, String>,
    #[serde(default)]
    pub goals: HashMap<String, clickup_tracker::summary::Goals>,
}

pub fn data_dir() -> Result<PathBuf> {
    let path = dirs::data_local_dir()
        .context(Message::new(Key::DataDirectory))?
        .join("ClickUpTracker");
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn load() -> Result<Preferences> {
    let path = data_dir()?.join("preferences.json");
    if !path.exists() {
        return Ok(Preferences::default());
    }
    serde_json::from_slice(&fs::read(path)?).context(Message::new(Key::ReadPreferences))
}

pub fn save(preferences: &Preferences) -> Result<()> {
    let dir = data_dir()?;
    let temporary = dir.join("preferences.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(preferences)?)?;
    // rename replaces an existing destination on macOS and Windows.
    fs::rename(temporary, dir.join("preferences.json"))?;
    Ok(())
}

fn credential() -> Result<keyring::Entry> {
    keyring::Entry::new("com.clickuptracker.desktop", "personal-api-token")
        .context(Message::new(Key::CredentialStore))
}

pub fn token() -> Result<Option<String>> {
    match credential()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => anyhow::bail!(Message::new(Key::ReadToken)),
    }
}

pub fn set_token(token: &str) -> Result<()> {
    credential()?
        .set_password(token)
        .map_err(|_| anyhow::anyhow!(Message::new(Key::SaveToken)))
}

pub fn forget_token() -> Result<()> {
    match credential()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => anyhow::bail!(Message::new(Key::DeleteToken)),
    }
}

#[cfg(test)]
mod locale_tests {
    use super::*;

    #[test]
    fn legacy_preferences_default_to_english() {
        let preferences: Preferences = serde_json::from_str(r#"{"workspace":"42"}"#).unwrap();
        assert_eq!(
            preferences.language,
            clickup_tracker::locale::Language::English
        );
    }

    #[test]
    fn language_round_trips_without_losing_preferences() {
        let preferences: Preferences =
            serde_json::from_str(r#"{"workspace":"42","language":"cs","last_list":{"42":"123"}}"#)
                .unwrap();
        let saved = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.language, clickup_tracker::locale::Language::Czech);
        assert_eq!(restored.workspace.as_deref(), Some("42"));
        assert_eq!(
            restored.last_list.get("42").map(String::as_str),
            Some("123")
        );
    }
}
