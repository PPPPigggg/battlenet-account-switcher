mod storm_core;

use std::sync::Mutex;
use storm_core::{AccountInfo, BattleNetCore, GroupInfo};
use tauri::{
    menu::{Menu, MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

const TRAY_ID: &str = "main";
const TRAY_SHOW_ID: &str = "show";
const TRAY_QUIT_ID: &str = "quit";
const TRAY_EMPTY_ACCOUNTS_ID: &str = "accounts-empty";
const TRAY_ACCOUNT_PREFIX: &str = "switch-account:";

struct AppState {
    core: Mutex<BattleNetCore>,
}

#[tauri::command]
fn get_accounts(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Vec<AccountInfo> {
    state
        .core
        .lock()
        .map(|mut core| core.get_accounts(&app))
        .unwrap_or_default()
}

#[tauri::command]
fn get_groups(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Vec<GroupInfo> {
    state
        .core
        .lock()
        .map(|mut core| core.get_groups(&app))
        .unwrap_or_default()
}

#[tauri::command]
fn create_group(
    name: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Option<GroupInfo> {
    state.core.lock().ok()?.create_group(&app, &name)
}

#[tauri::command]
fn rename_group(
    id: String,
    name: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    state
        .core
        .lock()
        .map(|mut core| core.rename_group(&app, &id, &name))
        .unwrap_or(false)
}

#[tauri::command]
fn delete_group(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    state
        .core
        .lock()
        .map(|mut core| core.delete_group(&app, &id))
        .unwrap_or(false)
}

#[tauri::command]
fn move_account_to_group(
    account_id: String,
    group_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    let ok = state
        .core
        .lock()
        .map(|mut core| core.move_account_to_group(&app, &account_id, &group_id))
        .unwrap_or(false);
    if ok {
        refresh_tray_menu(&app);
    }
    ok
}

#[tauri::command]
fn update_account_info(
    account_id: String,
    remark: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    let ok = state
        .core
        .lock()
        .map(|mut core| core.update_account_info(&app, &account_id, &remark))
        .unwrap_or(false);
    if ok {
        refresh_tray_menu(&app);
    }
    ok
}

#[tauri::command]
fn save_current_account_to_group(
    remark: String,
    group_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    let ok = state
        .core
        .lock()
        .map(|mut core| core.save_current_account_to_group(&app, &remark, &group_id))
        .unwrap_or(false);
    if ok {
        refresh_tray_menu(&app);
    }
    ok
}

#[tauri::command]
fn switch_account(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    let ok = state
        .core
        .lock()
        .map(|mut core| core.switch_account(&app, &id))
        .unwrap_or(false);
    if ok {
        refresh_tray_menu(&app);
    }
    ok
}

#[tauri::command]
fn delete_account(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    let ok = state
        .core
        .lock()
        .map(|mut core| core.delete_account(&app, &id))
        .unwrap_or(false);
    if ok {
        refresh_tray_menu(&app);
    }
    ok
}

#[tauri::command]
fn add_new_account(state: tauri::State<'_, AppState>) -> bool {
    state
        .core
        .lock()
        .map(|mut core| core.add_new_account())
        .unwrap_or(false)
}

#[tauri::command]
fn get_auto_start(state: tauri::State<'_, AppState>) -> bool {
    state
        .core
        .lock()
        .map(|core| core.get_auto_start())
        .unwrap_or(false)
}

#[tauri::command]
fn set_auto_start(enabled: bool, state: tauri::State<'_, AppState>) -> bool {
    state
        .core
        .lock()
        .map(|core| core.set_auto_start(enabled))
        .unwrap_or(false)
}

#[tauri::command]
fn get_close_to_tray(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    state
        .core
        .lock()
        .map(|core| core.get_close_to_tray(&app))
        .unwrap_or(true)
}

#[tauri::command]
fn set_close_to_tray(
    enabled: bool,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    state
        .core
        .lock()
        .map(|core| core.set_close_to_tray(&app, enabled))
        .unwrap_or(false)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState {
            core: Mutex::new(BattleNetCore::new()),
        })
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_accounts,
            get_groups,
            create_group,
            rename_group,
            delete_group,
            move_account_to_group,
            update_account_info,
            save_current_account_to_group,
            switch_account,
            delete_account,
            add_new_account,
            get_auto_start,
            set_auto_start,
            get_close_to_tray,
            set_close_to_tray
        ])
        .run(tauri::generate_context!())
        .expect("failed to run StormSwitch");
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("StormSwitch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => show_main_window(app),
            TRAY_QUIT_ID => app.exit(0),
            id if id.starts_with(TRAY_ACCOUNT_PREFIX) => {
                let account_id = id.trim_start_matches(TRAY_ACCOUNT_PREFIX);
                switch_account_from_tray(app, account_id);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let mut accounts = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .core
                .lock()
                .ok()
                .map(|mut core| core.get_accounts(app))
        })
        .unwrap_or_default();
    accounts.sort_by(|a, b| b.last_used.cmp(&a.last_used));

    let mut account_menu = SubmenuBuilder::new(app, "切换账号");
    if accounts.is_empty() {
        account_menu = account_menu.text(TRAY_EMPTY_ACCOUNTS_ID, "暂无账号");
    } else {
        for account in accounts {
            let label = if account.remark.trim().is_empty() {
                "未命名账号".to_string()
            } else {
                account.remark
            };
            account_menu = account_menu.text(format!("{TRAY_ACCOUNT_PREFIX}{}", account.id), label);
        }
    }

    MenuBuilder::new(app)
        .text(TRAY_SHOW_ID, "显示 StormSwitch")
        .item(&account_menu.build()?)
        .separator()
        .text(TRAY_QUIT_ID, "退出")
        .build()
}

fn refresh_tray_menu(app: &tauri::AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_tray_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn switch_account_from_tray(app: &tauri::AppHandle, account_id: &str) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    let switched = state
        .core
        .lock()
        .map(|mut core| core.switch_account(app, account_id))
        .unwrap_or(false);

    if switched {
        refresh_tray_menu(app);
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
