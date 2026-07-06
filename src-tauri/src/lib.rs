mod storm_core;

use std::sync::Mutex;
use storm_core::{AccountInfo, BattleNetCore, GroupInfo};

struct AppState {
    core: Mutex<BattleNetCore>,
}

#[tauri::command]
fn get_accounts(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Vec<AccountInfo> {
    state.core.lock().map(|mut core| core.get_accounts(&app)).unwrap_or_default()
}

#[tauri::command]
fn get_groups(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Vec<GroupInfo> {
    state.core.lock().map(|mut core| core.get_groups(&app)).unwrap_or_default()
}

#[tauri::command]
fn create_group(name: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Option<GroupInfo> {
    state.core.lock().ok()?.create_group(&app, &name)
}

#[tauri::command]
fn rename_group(id: String, name: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|mut core| core.rename_group(&app, &id, &name)).unwrap_or(false)
}

#[tauri::command]
fn delete_group(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|mut core| core.delete_group(&app, &id)).unwrap_or(false)
}

#[tauri::command]
fn move_account_to_group(
    account_id: String,
    group_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    state
        .core
        .lock()
        .map(|mut core| core.move_account_to_group(&app, &account_id, &group_id))
        .unwrap_or(false)
}

#[tauri::command]
fn update_account_info(
    account_id: String,
    remark: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    state
        .core
        .lock()
        .map(|mut core| core.update_account_info(&app, &account_id, &remark))
        .unwrap_or(false)
}

#[tauri::command]
fn save_current_account_to_group(
    remark: String,
    group_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> bool {
    state
        .core
        .lock()
        .map(|mut core| core.save_current_account_to_group(&app, &remark, &group_id))
        .unwrap_or(false)
}

#[tauri::command]
fn switch_account(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|mut core| core.switch_account(&app, &id)).unwrap_or(false)
}

#[tauri::command]
fn delete_account(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|mut core| core.delete_account(&app, &id)).unwrap_or(false)
}

#[tauri::command]
fn add_new_account(state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|mut core| core.add_new_account()).unwrap_or(false)
}

#[tauri::command]
fn get_auto_start(state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|core| core.get_auto_start()).unwrap_or(false)
}

#[tauri::command]
fn set_auto_start(enabled: bool, state: tauri::State<'_, AppState>) -> bool {
    state.core.lock().map(|core| core.set_auto_start(enabled)).unwrap_or(false)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState {
            core: Mutex::new(BattleNetCore::new()),
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
            set_auto_start
        ])
        .run(tauri::generate_context!())
        .expect("failed to run StormSwitch");
}
