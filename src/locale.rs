//! App-owned text and structured errors. User content is never translated.
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum Language {
    #[serde(rename = "cs")]
    Czech,
    #[default]
    #[serde(rename = "en", other)]
    English,
}

macro_rules! catalog {
    ($($key:ident => ($en:literal, $cs:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Key { $($key),+ }
        impl Key {
            pub const ALL: &'static [Self] = &[$(Self::$key),+];
        }
        impl Language {
            pub fn text(self, key: Key) -> &'static str {
                match (self, key) {
                    $((Self::English, Key::$key) => $en,
                      (Self::Czech, Key::$key) => $cs),+
                }
            }
        }
    };
}
catalog! {
    Increment => ("Increase", "Zvýšit"),
    Decrement => ("Decrease", "Snížit"),
    Clear => ("Clear", "Vymazat"),
    Cut => ("Cut", "Vyjmout"),
    Copy => ("Copy", "Kopírovat"),
    Paste => ("Paste", "Vložit"),
    SelectAll => ("Select all", "Vybrat vše"),
    SelectPlaceholder => ("Select…", "Vybrat…"),
    SearchPlaceholder => ("Search…", "Hledat…"),
    NoResults => ("No results", "Žádné výsledky"),
    Search => ("Search tasks or paste a link…", "Hledat task nebo vložit odkaz…"),
    TaskName => ("What will you work on?", "Na čem budeš pracovat?"),
    ConnectingClickUp => ("Connecting to ClickUp…", "Připojuju ClickUp…"),
    Welcome => ("Fewer clicks.\nMore done.", "Méně klikání.\nVíce hotovo."),
    TokenHelp => ("Enter your personal ClickUp API token. Then you can leave the browser closed.", "Vlož svůj osobní ClickUp API token. Pak už můžeš web nechat zavřený."),
    Connecting => ("Connecting…", "Připojuju…"),
    Connect => ("Connect ClickUp", "Připojit ClickUp"),
    FindToken => ("Where do I find my API token? ↗", "Kde najdu API token? ↗"),
    TokenPrivacy => ("Your token stays in the system credential store. The app communicates directly with ClickUp.", "Token zůstane v systémové schránce přihlašovacích údajů. Aplikace komunikuje přímo s ClickUpem."),
    Settings => ("Settings", "Nastavení"),
    ConnectedUser => ("Connected user: {0}", "Propojený uživatel: {0}"),
    Disconnect => ("Disconnect account and delete token", "Odpojit účet a smazat token"),
    Back => ("Back", "Zpět"),
    QuitApp => ("Quit application", "Ukončit aplikaci"),
    NewTask => ("New task", "Nový task"),
    ListHeading => ("LIST", "SEZNAM"),
    SelectList => ("Select a list…", "Vybrat seznam…"),
    LoadingLists => ("Loading lists…", "Načítám seznamy…"),
    NoLists => ("No lists loaded.", "Nejsou načtené žádné seznamy."),
    Working => ("Working…", "Pracuju…"),
    CreateTrack => ("Create and track", "Vytvořit a měřit"),
    UntitledTimer => ("Time without a task", "Čas bez tasku"),
    Ready => ("Ready for the next task", "Připraveno na další task"),
    Unverified => ("○  STATUS UNVERIFIED", "○  STAV NEOVĚŘENÝ"),
    Tracking => ("●  TRACKING NOW", "●  PRÁVĚ MĚŘÍŠ"),
    Stopped => ("○  STOPPED", "○  ZASTAVENO"),
    Stop => ("■ Stop", "■ Stop"),
    Recent => ("RECENT", "NEDÁVNÉ"),
    MyTasks => ("MY OPEN TASKS", "MOJE OTEVŘENÉ TASKY"),
    Results => ("SEARCH RESULTS", "VÝSLEDKY HLEDÁNÍ"),
    LoadingTasks => ("Loading tasks…", "Načítám tasky…"),
    NoTasks => ("No tasks. Paste a ClickUp link or ID and load it using the button below.", "Žádný task. Vlož ClickUp odkaz nebo ID a načti ho tlačítkem níže."),
    Lookup => ("+ Load task by link / ID", "+ Načíst task podle odkazu / ID"),
    AddTask => ("+ New task", "+ Nový task"),
    Connected => ("Connected to ClickUp", "Připojeno ke ClickUpu"),
    Summary => ("Summary", "Shrnutí"),
    Quit => ("Quit", "Ukončit"),
    Goal => ("goal {0} h", "cíl {0} h"),
    GoalExtra => ("Goal reached · extra {0}", "Cíl splněn · navíc {0}"),
    GoalReached => ("Goal reached", "Cíl splněn"),
    GoalRemaining => ("Remaining to goal: {0}", "Do cíle zbývá {0}"),
    CurrentWorkspace => ("Current workspace", "Aktuální workspace"),
    ThisMonth => ("This month", "Tento měsíc"),
    ThisWeek => ("This week", "Tento týden"),
    LastVerified => ("Showing the last verified data.", "Zobrazuju poslední ověřené údaje."),
    LoadingHours => ("Loading hours from ClickUp…", "Načítám hodiny z ClickUpu…"),
    HoursUnavailable => ("Hours could not be loaded yet.", "Hodiny se zatím nepodařilo načíst."),
    MyGoals => ("My goals", "Moje cíle"),
    MonthlyHours => ("Monthly · hours", "Měsíčně · hodiny"),
    MonthlyGoal => ("Monthly goal in hours", "Měsíční cíl v hodinách"),
    WeeklyHours => ("Weekly · hours", "Týdně · hodiny"),
    WeeklyGoal => ("Weekly goal in hours", "Týdenní cíl v hodinách"),
    Saved => ("Saved ✓", "Uloženo ✓"),
    SaveGoals => ("Save goals", "Uložit cíle"),
    Refresh => ("Refresh", "Obnovit"),
    SummaryHelp => ("Weeks start on Monday. Includes the running timer. Goals apply to this workspace.", "Týden od pondělí. Včetně běžícího timeru. Cíle platí pro tento workspace."),
    Language => ("Language", "Jazyk"),
    English => ("English", "Angličtina"),
    Czech => ("Czech", "Čeština"),
    Workspace => ("Workspace", "Workspace"),
    TrayStopped => ("ClickUp Tracker • stopped", "ClickUp Tracker • zastaveno"),
    WorkerDisconnected => ("The connection to the app was lost. Quit and restart it.", "Spojení s aplikací se přerušilo. Ukonči ji a spusť znovu."),
    AlreadyRunning => ("ClickUp Tracker is already running. Click the timer icon in the tray.", "ClickUp Tracker už běží. Klikni na ikonku časovače v liště."),
    DataDirectory => ("Cannot find the application data directory.", "Nelze najít složku aplikace."),
    ReadPreferences => ("Cannot read settings. Check preferences.json in the application data directory.", "Nastavení nelze přečíst. Zkontroluj preferences.json ve složce aplikace."),
    CredentialStore => ("The system credential store is unavailable.", "Systémová schránka přihlašovacích údajů není dostupná."),
    ReadToken => ("Cannot read the token from the system credential store. Unlock it or reconnect your account.", "Token nelze načíst ze systémové schránky. Odemkni ji nebo připoj účet znovu."),
    SaveToken => ("Cannot save the token to the system credential store. Unlock it and try again.", "Token nelze uložit do systémové schránky. Odemkni ji a zkus to znovu."),
    DeleteToken => ("Could not delete the token from the system credential store.", "Token se nepodařilo odstranit ze systémové schránky."),
    NoWorkspaces => ("This account has no available workspaces.", "Účet nemá žádný dostupný workspace."),
    ConnectFirst => ("Connect a ClickUp account first.", "Nejprve připoj ClickUp účet."),
    ChooseWorkspace => ("Select a workspace.", "Vyber workspace."),
    SavePreferences => ("Could not save settings to disk.", "Nastavení se nepodařilo uložit na disk."),
    UnknownWorkspace => ("Unknown workspace.", "Neznámý workspace."),
    MissingUser => ("No signed-in user.", "Chybí přihlášený uživatel."),
    SaveGoalsError => ("Could not save goals to disk.", "Cíle se nepodařilo uložit na disk."),
    ChooseWorkspaceList => ("Select a list in the selected workspace.", "Vyber seznam ve vybraném workspace."),
    CreatedPreferencesError => ("The task was created and is in recents. Settings could not be saved; tracking has not started.", "Task je vytvořený a najdeš ho mezi nedávnými. Nastavení se nepodařilo uložit; měření nového tasku se ještě nespustilo."),
    CreatedTrackingError => ("The task was created, but tracking could not be confirmed. Find it in recents.", "Task je vytvořený, ale měření se nepodařilo potvrdit. Najdeš ho mezi nedávnými."),
    TaskLinkHost => ("Paste a link from app.clickup.com.", "Vlož odkaz z app.clickup.com."),
    TaskLinkPath => ("The link must contain /t/TASK-ID.", "Odkaz musí obsahovat /t/ID-tasku."),
    TaskId => ("Paste a ClickUp link or a regular task ID (not a custom ID such as PROJ-123).", "Vlož ClickUp odkaz nebo běžné ID tasku (ne vlastní ID typu PROJ-123)."),
    EnterTaskName => ("Enter a task name.", "Zadej název tasku."),
    ValidList => ("Select a valid list.", "Vyber platný seznam."),
    CreateUnconfirmed => ("Task creation could not be confirmed. Check ClickUp before retrying to avoid creating a duplicate.", "Vytvoření tasku se nepodařilo potvrdit. Před opakováním zkontroluj ClickUp, aby nevznikla kopie."),
    EnterToken => ("Enter your API token.", "Vlož svůj API token."),
    Unavailable => ("ClickUp is unavailable. Check your connection and refresh; requests are not retried automatically.", "ClickUp není dostupný. Zkontroluj připojení a obnov stav; požadavek se neopakuje automaticky."),
    InvalidToken => ("Invalid or revoked API token. Reconnect your account.", "Neplatný nebo zrušený API token. Připoj účet znovu."),
    Forbidden => ("ClickUp denied access (HTTP 403). Check permissions and that Time Tracking is enabled in the workspace.", "ClickUp nepovolil přístup (HTTP 403). Ověř oprávnění a zapnutý Time Tracking ve workspace."),
    RateLimited => ("ClickUp limited requests (HTTP 429). Wait a moment and refresh.", "ClickUp omezil počet požadavků (HTTP 429). Chvíli počkej a obnov stav."),
    HttpError => ("ClickUp rejected the request (HTTP {0}). Refresh; if it persists, check task access and Time Tracking in the workspace.", "ClickUp požadavek odmítl (HTTP {0}). Obnov stav; při opakování ověř přístup k tasku a Time Tracking ve workspace."),
    UnexpectedResponse => ("ClickUp returned an unexpected response. Refresh before tracking again.", "ClickUp vrátil neočekávanou odpověď. Obnov stav před dalším měřením."),
    TaskWorkspace => ("This task does not belong to the selected workspace. Select the correct workspace and load it again.", "Task nepatří do vybraného workspace. Vyber správný workspace a načti ho znovu."),
    TimerWorkspace => ("The timer is running in another workspace. Select that workspace before changing tracking.", "Timer běží v jiném workspace. Vyber jeho workspace, než změníš měření."),
    StartUnconfirmed => ("ClickUp has not confirmed starting the selected task. Refresh before trying again.", "ClickUp zatím nepotvrdil spuštění vybraného tasku. Obnov stav před dalším pokusem."),
    StartUnverified => ("The start request was sent, but the timer status could not be verified.", "Spuštění bylo odesláno, ale stav timeru nelze ověřit."),
    GoalNumber => ("A goal must be a number of hours, such as 80 or 20.5.", "Cíl musí být počet hodin, například 80 nebo 20,5."),
    GoalRange => ("A goal must be greater than 0 and at most 10,000 hours.", "Cíl musí být větší než 0 a nejvýše 10 000 hodin."),
    PeriodStart => ("Cannot determine the start of the period in local time.", "Nelze určit začátek období v místním čase."),
}

impl Language {
    pub fn format(self, key: Key, values: &[&str]) -> String {
        // Split the catalog template first so inserted user content remains literal.
        let template = self.text(key);
        let mut output = String::new();
        let mut remaining = template;
        while let Some(start) = remaining.find('{') {
            output.push_str(&remaining[..start]);
            let Some(end) = remaining[start..].find('}') else {
                break;
            };
            let token = &remaining[start + 1..start + end];
            if let Some(value) = token.parse::<usize>().ok().and_then(|i| values.get(i)) {
                output.push_str(value);
            } else {
                output.push_str(&remaining[start..=start + end]);
            }
            remaining = &remaining[start + end + 1..];
        }
        output.push_str(remaining);
        output
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    key: Option<Key>,
    values: Vec<String>,
    detail: Option<String>,
}
impl Message {
    pub fn new(key: Key) -> Self {
        Self {
            key: Some(key),
            values: Vec::new(),
            detail: None,
        }
    }
    pub fn with_value(key: Key, value: impl ToString) -> Self {
        Self {
            key: Some(key),
            values: vec![value.to_string()],
            detail: None,
        }
    }
    pub fn from_error(error: anyhow::Error) -> Self {
        error
            .downcast_ref::<Self>()
            .cloned()
            .unwrap_or_else(|| Self {
                key: None,
                values: Vec::new(),
                detail: Some(error.to_string()),
            })
    }
    pub fn render(&self, language: Language) -> String {
        match self.key {
            Some(key) => language.format(
                key,
                &self.values.iter().map(String::as_str).collect::<Vec<_>>(),
            ),
            None => self.detail.clone().unwrap_or_default(),
        }
    }
}
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(Language::English))
    }
}
impl std::error::Error for Message {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_language_falls_back_to_english() {
        assert_eq!(
            serde_json::from_str::<Language>("\"de\"").unwrap(),
            Language::English
        );
    }
    #[test]
    fn every_catalog_entry_exists_in_both_languages() {
        for &key in Key::ALL {
            for language in [Language::English, Language::Czech] {
                assert!(
                    !language.text(key).trim().is_empty(),
                    "{language:?}: {key:?}"
                );
            }
            let placeholders = |text: &str| text.matches("{0}").count();
            assert_eq!(
                placeholders(Language::English.text(key)),
                placeholders(Language::Czech.text(key)),
                "{key:?}"
            );
        }
    }
    #[test]
    fn errors_switch_language_without_being_recreated() {
        let error = Message::from_error(anyhow::Error::new(Message::new(Key::InvalidToken)));
        assert_eq!(
            error.render(Language::English),
            Language::English.text(Key::InvalidToken)
        );
        assert_eq!(
            error.render(Language::Czech),
            Language::Czech.text(Key::InvalidToken)
        );
        assert_eq!(error.to_string(), error.render(Language::English));
    }
    #[test]
    fn user_content_is_preserved_literally() {
        let name = "Příliš {0} {1} žluťoučký";
        assert!(
            Language::Czech
                .format(Key::ConnectedUser, &[name])
                .ends_with(name)
        );
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;
    #[test]
    fn localized_context_is_preserved_in_the_ui() {
        let error =
            anyhow::anyhow!("transport detail").context(Message::new(Key::CreatedTrackingError));
        let message = Message::from_error(error);
        assert_eq!(
            message.render(Language::Czech),
            Language::Czech.text(Key::CreatedTrackingError)
        );
    }
}

#[cfg(feature = "desktop")]
const COMPONENT_KEYS: &[(&str, Key)] = &[
    ("Input.Increment", Key::Increment),
    ("Input.Decrement", Key::Decrement),
    ("Input.Clear", Key::Clear),
    ("Input.Cut", Key::Cut),
    ("Input.Copy", Key::Copy),
    ("Input.Paste", Key::Paste),
    ("Input.Select All", Key::SelectAll),
    ("Select.placeholder", Key::SelectPlaceholder),
    ("List.search_placeholder", Key::SearchPlaceholder),
];

/// Register component translations once before constructing any controls.
#[cfg(feature = "desktop")]
pub fn init_components() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let mut backend = rust_i18n::SimpleBackend::new();
        for (code, language) in [("en", Language::English), ("cs", Language::Czech)] {
            backend.add_translations(
                code.into(),
                COMPONENT_KEYS
                    .iter()
                    .map(|(name, key)| {
                        (format!("tracker.{name}").into(), language.text(*key).into())
                    })
                    .collect(),
            );
        }
        gpui_kit::component::_rust_i18n_extend(Box::leak(Box::new(backend)), "tracker");
    });
}

#[cfg(feature = "desktop")]
pub fn set_component_language(language: Language) {
    gpui_kit::component::set_locale(match language {
        Language::English => "en",
        Language::Czech => "cs",
    });
}

#[cfg(all(test, feature = "desktop"))]
mod component_tests {
    use super::*;
    #[test]
    fn component_catalog_translates_controls_and_switches_immediately() {
        init_components();
        for &(name, key) in COMPONENT_KEYS {
            assert_eq!(
                gpui_kit::component::_rust_i18n_translate("cs", name),
                Language::Czech.text(key)
            );
            assert_eq!(
                gpui_kit::component::_rust_i18n_translate("en", name),
                Language::English.text(key)
            );
        }
        set_component_language(Language::Czech);
        assert_eq!(&*gpui_kit::component::locale(), "cs");
        set_component_language(Language::English);
        assert_eq!(&*gpui_kit::component::locale(), "en");
    }
}
