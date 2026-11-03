#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

extern crate gpui_kit as gpui;
use clickup_tracker::locale::{Key, Language, Message};

mod storage;
mod summary_ui;
mod ui;
mod worker;

use gpui::*;
use gpui_tray::{Icon, Tray};
use std::{fs::OpenOptions, sync::mpsc};
use worker::{Command, Snapshot};

struct State {
    snapshot: Snapshot,
    commands: mpsc::Sender<Command>,
    busy: bool,
    tray: Tray,
}
impl Global for State {}

fn send(command: Command, cx: &mut App) {
    let state = cx.global_mut::<State>();
    if state.busy {
        return;
    }
    state.busy = true;
    state.snapshot.error = None;
    if state.commands.send(command).is_err() {
        state.busy = false;
        state.snapshot.error = Some(Message::new(Key::WorkerDisconnected));
    }
    cx.refresh_windows();
}

fn tray_icon(running: bool) -> gpui_tray::Result<Icon> {
    let mut rgba = vec![0u8; 32 * 32 * 4];
    for y in 0..32i32 {
        for x in 0..32i32 {
            let d = (x - 16).pow(2) + (y - 17).pow(2);
            let ring = (100..=169).contains(&d);
            let hand = (15..=17).contains(&x) && (8..=18).contains(&y)
                || (16..=23).contains(&x) && (16..=18).contains(&y);
            let top = (13..=19).contains(&x) && (0..=3).contains(&y);
            if ring || hand || top {
                let color = if cfg!(target_os = "macos") {
                    [0, 0, 0, 255]
                } else if running {
                    [49, 205, 146, 255]
                } else {
                    [132, 150, 175, 255]
                };
                let offset = ((y * 32 + x) * 4) as usize;
                rgba[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
    let builder = Icon::from_rgba(rgba, 32, 32);
    #[cfg(target_os = "macos")]
    let builder = builder.scale_factor(2.0).template(true);
    builder.build()
}

fn toggle_popup(cx: &mut App) {
    if let Some(window) = cx.windows().first().copied() {
        let _ = window.update(cx, |_, window, _| window.remove_window());
        return;
    }
    show_popup(cx);
}

fn show_popup(cx: &mut App) {
    let Some(display) = cx.primary_display() else {
        return;
    };
    let bounds = display.bounds();
    let size = size(px(400.), px(600.).min(bounds.size.height - px(60.)));
    let x = bounds.origin.x + bounds.size.width - size.width - px(12.);
    let y = if cfg!(target_os = "macos") {
        bounds.origin.y + px(38.)
    } else {
        bounds.origin.y + bounds.size.height - size.height - px(56.)
    };
    let result = gpui_kit::open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(x, y), size))),
            titlebar: None,
            kind: WindowKind::PopUp,
            is_resizable: false,
            is_minimizable: false,
            is_movable: false,
            app_id: Some("com.clickuptracker.desktop".into()),
            ..Default::default()
        },
        cx,
        |window, cx| cx.new(|cx| ui::Tracker::new(window, cx)),
    );
    if let Err(error) = result {
        eprintln!("Cannot open tracker window: {error}");
    }
    cx.activate(true);
}

fn main() -> anyhow::Result<()> {
    let language = Language::English;
    // The open file owns the lock until the application exits.
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(storage::data_dir()?.join("instance.lock"))?;
    if fs2::FileExt::try_lock_exclusive(&lock).is_err() {
        eprintln!("{}", language.text(Key::AlreadyRunning));
        return Ok(());
    }
    let application = gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .with_quit_mode(QuitMode::Explicit);
    application.on_reopen(|cx| {
        if cx.try_global::<State>().is_some() {
            if let Some(window) = cx.windows().first().copied() {
                let _ = window.update(cx, |_, window, _| window.activate_window());
            } else {
                show_popup(cx);
            }
        }
    });
    application.run(move |cx| {
        gpui_kit::init(cx);
        clickup_tracker::locale::init_components();
        clickup_tracker::locale::set_component_language(Language::English);
        cx.bind_keys([
            KeyBinding::new("escape", ui::Dismiss, Some("Tracker")),
            KeyBinding::new("cmd-q", ui::Quit, Some("Tracker")),
            KeyBinding::new("ctrl-q", ui::Quit, Some("Tracker")),
        ]);
        #[cfg(target_os = "macos")]
        cx.set_activation_policy(ActivationPolicy::Accessory);

        let tray = match Tray::builder()
            .icon(tray_icon(false).expect("Invalid tray icon"))
            .tooltip(language.text(Key::TrayStopped))
            .on_activate_with(|_, cx| toggle_popup(cx))
            .build(cx)
        {
            Ok(tray) => tray,
            Err(error) => {
                eprintln!("Cannot create tray: {error}");
                cx.quit();
                return;
            }
        };
        let (commands, events) = worker::spawn();
        cx.set_global(State {
            snapshot: Snapshot::default(),
            commands,
            busy: true,
            tray,
        });
        cx.spawn(async move |cx| {
            while let Ok(event) = events.recv().await {
                cx.update(|cx| {
                    let state = cx.global_mut::<State>();
                    if event.completed {
                        state.busy = false;
                    }
                    state.snapshot = event.snapshot;
                    let language = state.snapshot.language;
                    clickup_tracker::locale::set_component_language(language);
                    let tray = state.tray.clone();
                    let current = state.snapshot.current.as_ref();
                    let running = current.is_some();
                    let tooltip = match current {
                        Some(entry) => format!(
                            "ClickUp • {}",
                            entry
                                .task
                                .as_ref()
                                .map(|t| t.name.as_str())
                                .unwrap_or(language.text(Key::UntitledTimer))
                        ),
                        None => language.text(Key::TrayStopped).into(),
                    };
                    let title = if running { "●" } else { "" };
                    let _ = tray.set_title(Some(title), cx);
                    let _ = tray.set_tooltip(Some(tooltip), cx);
                    if let Ok(icon) = tray_icon(running) {
                        let _ = tray.set_icon(icon, cx);
                    }
                    cx.refresh_windows();
                });
            }
        })
        .detach();
        show_popup(cx);
    });
    drop(lock);
    Ok(())
}
