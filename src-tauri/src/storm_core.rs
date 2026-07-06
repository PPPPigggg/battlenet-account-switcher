use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashSet, env, fs, path::PathBuf, thread, time::Duration};
#[cfg(windows)]
use std::{io, process::Command};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const DEFAULT_GROUP_ID: &str = "default";
const DEFAULT_GROUP_NAME: &str = "默认分组";
const APP_NAME: &str = "StormSwitch";
const STORE_FILE: &str = "settings.json";
const ACCOUNTS_KEY: &str = "accounts";
const GROUPS_KEY: &str = "groups";
const BATTLE_NET_CONFIG_FILE: &str = "Battle.net.config";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AccountInfo {
    pub id: String,
    pub remark: String,
    #[serde(default)]
    pub username: String,
    pub last_used: String,
    pub group_id: String,
    #[serde(default)]
    pub logged_in: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

pub struct BattleNetCore {
    app_data_path: PathBuf,
    data_dir: PathBuf,
    config_file_path: PathBuf,
}

impl BattleNetCore {
    pub fn new() -> Self {
        let app_data_path = battle_net_app_data_path();
        let data_dir = local_data_dir().join(APP_NAME).join("Data");
        let config_file_path = app_data_path.join(BATTLE_NET_CONFIG_FILE);

        let core = Self {
            app_data_path,
            data_dir,
            config_file_path,
        };

        let _ = fs::create_dir_all(&core.data_dir);
        core
    }

    pub fn get_accounts(&mut self, app: &tauri::AppHandle) -> Vec<AccountInfo> {
        let mut accounts = self.read_accounts(app);
        if self.normalize_accounts(app, &mut accounts) {
            let _ = self.save_accounts(app, &accounts);
        }
        self.mark_logged_in_accounts(&mut accounts);
        accounts
    }

    pub fn get_groups(&mut self, app: &tauri::AppHandle) -> Vec<GroupInfo> {
        let mut groups = self.read_groups(app);
        self.normalize_groups(&mut groups);
        let _ = self.save_groups(app, &groups);
        groups
    }

    pub fn create_group(&mut self, app: &tauri::AppHandle, name: &str) -> Option<GroupInfo> {
        let name = normalize_name(name);
        if name.is_empty() {
            return None;
        }

        let mut groups = self.read_groups(app);
        if let Some(existing) = groups
            .iter()
            .find(|group| group.name.eq_ignore_ascii_case(&name))
        {
            return Some(existing.clone());
        }

        let group = GroupInfo {
            id: Uuid::new_v4().simple().to_string(),
            name,
            created_at: now_string(),
        };
        groups.push(group.clone());
        let _ = self.save_groups(app, &groups);
        Some(group)
    }

    pub fn rename_group(&mut self, app: &tauri::AppHandle, id: &str, name: &str) -> bool {
        if id == DEFAULT_GROUP_ID {
            return false;
        }

        let name = normalize_name(name);
        if name.is_empty() {
            return false;
        }

        let mut groups = self.read_groups(app);
        if groups
            .iter()
            .any(|group| group.id != id && group.name.eq_ignore_ascii_case(&name))
        {
            return false;
        }

        let Some(group) = groups.iter_mut().find(|group| group.id == id) else {
            return false;
        };

        group.name = name;
        self.save_groups(app, &groups)
    }

    pub fn delete_group(&mut self, app: &tauri::AppHandle, id: &str) -> bool {
        if id.is_empty() || id == DEFAULT_GROUP_ID {
            return false;
        }

        let mut groups = self.read_groups(app);
        let original_len = groups.len();
        groups.retain(|group| group.id != id);
        if groups.len() == original_len || !self.save_groups(app, &groups) {
            return false;
        }

        let mut accounts = self.get_accounts(app);
        let mut changed = false;
        for account in &mut accounts {
            if account.group_id == id {
                account.group_id = DEFAULT_GROUP_ID.to_string();
                changed = true;
            }
        }
        !changed || self.save_accounts(app, &accounts)
    }

    pub fn move_account_to_group(
        &mut self,
        app: &tauri::AppHandle,
        account_id: &str,
        group_id: &str,
    ) -> bool {
        let target_group_id = self.ensure_valid_group_id(app, group_id);
        let mut accounts = self.get_accounts(app);
        let Some(account) = accounts.iter_mut().find(|account| account.id == account_id) else {
            return false;
        };

        account.group_id = target_group_id;
        self.save_accounts(app, &accounts)
    }

    pub fn update_account_info(
        &mut self,
        app: &tauri::AppHandle,
        account_id: &str,
        remark: &str,
    ) -> bool {
        let mut accounts = self.get_accounts(app);
        let Some(account) = accounts.iter_mut().find(|account| account.id == account_id) else {
            return false;
        };

        account.remark = fallback_account_name(remark);
        self.save_accounts(app, &accounts)
    }

    pub fn save_current_account_to_group(
        &mut self,
        app: &tauri::AppHandle,
        remark: &str,
        group_id: &str,
    ) -> bool {
        if !self.config_file_path.exists() {
            return false;
        }

        let account_id = Uuid::new_v4().simple().to_string();
        let account_dir = self.account_dir(&account_id);
        if fs::create_dir_all(&account_dir).is_err() {
            return false;
        }

        if fs::copy(
            &self.config_file_path,
            account_dir.join(BATTLE_NET_CONFIG_FILE),
        )
        .is_err()
        {
            return false;
        }

        let mut accounts = self.get_accounts(app);
        accounts.push(AccountInfo {
            id: account_id,
            remark: fallback_account_name(remark),
            username: self.current_account_name().unwrap_or_default(),
            last_used: now_string(),
            group_id: self.ensure_valid_group_id(app, group_id),
            logged_in: false,
        });

        self.save_accounts(app, &accounts)
    }

    pub fn switch_account(&mut self, app: &tauri::AppHandle, id: &str) -> bool {
        if !is_safe_id(id) {
            return false;
        }

        let saved_config = self.account_dir(id).join(BATTLE_NET_CONFIG_FILE);
        if !saved_config.exists() {
            return false;
        }

        if !self.is_multiple_instances_enabled() {
            kill_battle_net_processes();
            thread::sleep(Duration::from_millis(1500));
        }

        if self.config_file_path.exists() && fs::remove_file(&self.config_file_path).is_err() {
            return false;
        }

        if fs::create_dir_all(&self.app_data_path).is_err() {
            return false;
        }

        if fs::copy(saved_config, &self.config_file_path).is_err() {
            return false;
        }

        let mut accounts = self.get_accounts(app);
        if let Some(account) = accounts.iter_mut().find(|account| account.id == id) {
            account.last_used = now_string();
            let _ = self.save_accounts(app, &accounts);
        }

        launch_battle_net();
        true
    }

    pub fn delete_account(&mut self, app: &tauri::AppHandle, id: &str) -> bool {
        if !is_safe_id(id) {
            return false;
        }

        let mut accounts = self.get_accounts(app);
        let original_len = accounts.len();
        accounts.retain(|account| account.id != id);
        if accounts.len() == original_len || !self.save_accounts(app, &accounts) {
            return false;
        }

        let account_dir = self.account_dir(id);
        if account_dir.exists() {
            let _ = fs::remove_dir_all(account_dir);
        }
        true
    }

    pub fn add_new_account(&mut self) -> bool {
        if !self.is_multiple_instances_enabled() {
            kill_battle_net_processes();
            thread::sleep(Duration::from_millis(1500));
        }

        if self.config_file_path.exists() {
            let _ = fs::remove_file(&self.config_file_path);
        }

        launch_battle_net();
        true
    }

    pub fn get_auto_start(&self) -> bool {
        platform_get_auto_start()
    }

    pub fn set_auto_start(&self, enabled: bool) -> bool {
        platform_set_auto_start(enabled)
    }

    fn read_accounts(&self, app: &tauri::AppHandle) -> Vec<AccountInfo> {
        read_store_value(app, ACCOUNTS_KEY).unwrap_or_default()
    }

    fn save_accounts(&self, app: &tauri::AppHandle, accounts: &[AccountInfo]) -> bool {
        write_store_value(app, ACCOUNTS_KEY, accounts)
    }

    fn read_groups(&self, app: &tauri::AppHandle) -> Vec<GroupInfo> {
        let mut groups = read_store_value(app, GROUPS_KEY).unwrap_or_default();
        self.normalize_groups(&mut groups);
        groups
    }

    fn save_groups(&self, app: &tauri::AppHandle, groups: &[GroupInfo]) -> bool {
        let mut normalized = groups.to_vec();
        self.normalize_groups(&mut normalized);
        write_store_value(app, GROUPS_KEY, &normalized)
    }

    fn normalize_groups(&self, groups: &mut Vec<GroupInfo>) {
        groups.retain(|group| !group.id.trim().is_empty());

        if let Some(index) = groups.iter().position(|group| group.id == DEFAULT_GROUP_ID) {
            let mut default_group = groups.remove(index);
            default_group.name = DEFAULT_GROUP_NAME.to_string();
            default_group.created_at = "0001-01-01T00:00:00".to_string();
            groups.insert(0, default_group);
        } else {
            groups.insert(0, default_group());
        }

        for group in groups {
            group.name = normalize_name(&group.name);
            if group.name.is_empty() {
                group.name = "未命名分组".to_string();
            }
        }
    }

    fn normalize_accounts(&mut self, app: &tauri::AppHandle, accounts: &mut [AccountInfo]) -> bool {
        let groups = self.read_groups(app);
        let group_ids: HashSet<&str> = groups.iter().map(|group| group.id.as_str()).collect();
        let mut changed = false;

        for account in accounts {
            if account.group_id.is_empty() || !group_ids.contains(account.group_id.as_str()) {
                account.group_id = DEFAULT_GROUP_ID.to_string();
                changed = true;
            }
            if account.remark.trim().is_empty() {
                account.remark = "未命名账号".to_string();
                changed = true;
            }
            if account.username.trim().is_empty() {
                if let Some(username) = self.saved_account_name(&account.id) {
                    account.username = username;
                    changed = true;
                }
            }
        }

        changed
    }

    fn ensure_valid_group_id(&mut self, app: &tauri::AppHandle, group_id: &str) -> String {
        if group_id.is_empty() {
            return DEFAULT_GROUP_ID.to_string();
        }

        let groups = self.read_groups(app);
        if groups.iter().any(|group| group.id == group_id) {
            group_id.to_string()
        } else {
            DEFAULT_GROUP_ID.to_string()
        }
    }

    fn account_dir(&self, id: &str) -> PathBuf {
        self.data_dir.join(id)
    }

    fn is_multiple_instances_enabled(&self) -> bool {
        fs::read_to_string(&self.config_file_path)
            .ok()
            .and_then(|content| battle_net_single_instance_value(&content))
            .map(|single_instance| !single_instance)
            .unwrap_or(false)
    }

    fn current_account_name(&self) -> Option<String> {
        fs::read_to_string(&self.config_file_path)
            .ok()
            .and_then(|content| battle_net_saved_account_names(&content).into_iter().next())
    }

    fn saved_account_name(&self, id: &str) -> Option<String> {
        let saved_config = self.account_dir(id).join(BATTLE_NET_CONFIG_FILE);
        fs::read_to_string(saved_config)
            .ok()
            .and_then(|content| battle_net_saved_account_names(&content).into_iter().next())
    }

    fn mark_logged_in_accounts(&self, accounts: &mut [AccountInfo]) {
        let logged_in_names = fs::read_to_string(&self.config_file_path)
            .ok()
            .map(|content| {
                battle_net_saved_account_names(&content)
                    .into_iter()
                    .map(|name| normalize_account_name(&name))
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        for account in accounts {
            account.logged_in = !account.username.is_empty()
                && logged_in_names.contains(&normalize_account_name(&account.username));
        }
    }
}

fn read_store_value<T>(app: &tauri::AppHandle, key: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let store = app.store(STORE_FILE).ok()?;
    let value = store.get(key)?;
    serde_json::from_value(value.clone()).ok()
}

fn write_store_value<T>(app: &tauri::AppHandle, key: &str, value: &T) -> bool
where
    T: Serialize + ?Sized,
{
    let Ok(store) = app.store(STORE_FILE) else {
        return false;
    };
    store.set(key.to_string(), json!(value));
    store.save().is_ok()
}

fn default_group() -> GroupInfo {
    GroupInfo {
        id: DEFAULT_GROUP_ID.to_string(),
        name: DEFAULT_GROUP_NAME.to_string(),
        created_at: "0001-01-01T00:00:00".to_string(),
    }
}

fn fallback_account_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "未命名账号".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_name(value: &str) -> String {
    value.trim().to_string()
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn battle_net_single_instance_value(content: &str) -> Option<bool> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    find_single_instance_value(&value)
}

fn battle_net_saved_account_names(content: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };

    let Some(value) = find_saved_account_names_value(&value) else {
        return Vec::new();
    };

    parse_account_names(value)
}

fn find_single_instance_value(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(value) = map.get("SingleInstance") {
                return parse_boolish_value(value);
            }

            map.values().find_map(find_single_instance_value)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_single_instance_value),
        _ => None,
    }
}

fn find_saved_account_names_value(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(value) = map.get("SavedAccountNames") {
                return Some(value);
            }

            map.values().find_map(find_saved_account_names_value)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_saved_account_names_value),
        _ => None,
    }
}

fn parse_account_names(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(value) => split_account_names(value),
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str())
            .flat_map(split_account_names)
            .collect(),
        _ => Vec::new(),
    }
}

fn split_account_names(value: &str) -> Vec<String> {
    value
        .split([',', ';', '\n', '\r', '\t'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn normalize_account_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn parse_boolish_value(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(value) => Some(*value),
        serde_json::Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn battle_net_app_data_path() -> PathBuf {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Battle.net")
}

fn local_data_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn kill_battle_net_processes() {
    kill_process("Battle.net");
    kill_process("Agent");
}

#[cfg(windows)]
fn kill_process(name: &str) {
    let image_name = format!("{name}.exe");
    let _ = Command::new("taskkill")
        .args(["/IM", &image_name, "/F", "/T"])
        .creation_flags(0x08000000)
        .status();
}

#[cfg(not(windows))]
fn kill_process(_name: &str) {}

#[cfg(windows)]
fn launch_battle_net() {
    if let Some(exe_path) = battle_net_exe_path() {
        let _ = Command::new(exe_path).spawn();
    }
}

#[cfg(not(windows))]
fn launch_battle_net() {}

#[cfg(windows)]
fn battle_net_exe_path() -> Option<PathBuf> {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

    let default_path = PathBuf::from(r"C:\Program Files (x86)\Battle.net\Battle.net.exe");
    if default_path.exists() {
        return Some(default_path);
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Battle.net")
        .ok()?;
    let install_location: String = key.get_value("InstallLocation").ok()?;
    let exe_path = PathBuf::from(install_location).join("Battle.net.exe");
    exe_path.exists().then_some(exe_path)
}

#[cfg(windows)]
fn platform_get_auto_start() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .ok()
        .and_then(|key| key.get_value::<String, _>(APP_NAME).ok())
        .is_some()
}

#[cfg(not(windows))]
fn platform_get_auto_start() -> bool {
    false
}

#[cfg(windows)]
fn platform_set_auto_start(enabled: bool) -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") else {
        return false;
    };

    if enabled {
        let Ok(exe_path) = env::current_exe() else {
            return false;
        };
        key.set_value(APP_NAME, &exe_path.to_string_lossy().to_string())
            .is_ok()
    } else {
        key.delete_value(APP_NAME)
            .or_else(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .is_ok()
    }
}

#[cfg(not(windows))]
fn platform_set_auto_start(_enabled: bool) -> bool {
    false
}

#[cfg(windows)]
trait CommandExtHidden {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl CommandExtHidden for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt;
        CommandExt::creation_flags(self, flags)
    }
}

#[cfg(test)]
mod tests {
    use super::{battle_net_saved_account_names, battle_net_single_instance_value};

    #[test]
    fn reads_string_single_instance_value() {
        let content = r#"{"Client":{"SingleInstance":"false"}}"#;

        assert_eq!(battle_net_single_instance_value(content), Some(false));
    }

    #[test]
    fn reads_boolean_single_instance_value() {
        let content = r#"{"Client":{"SingleInstance":true}}"#;

        assert_eq!(battle_net_single_instance_value(content), Some(true));
    }

    #[test]
    fn returns_none_when_single_instance_is_missing() {
        let content = r#"{"Client":{"Locale":"zhCN"}}"#;

        assert_eq!(battle_net_single_instance_value(content), None);
    }

    #[test]
    fn reads_saved_account_names() {
        let content = r#"{"Client":{"SavedAccountNames":"one@example.com,two@example.com"}}"#;

        assert_eq!(
            battle_net_saved_account_names(content),
            vec!["one@example.com", "two@example.com"]
        );
    }

    #[test]
    fn reads_saved_account_names_from_arrays() {
        let content = r#"{"Client":{"SavedAccountNames":["one@example.com", "two@example.com"]}}"#;

        assert_eq!(
            battle_net_saved_account_names(content),
            vec!["one@example.com", "two@example.com"]
        );
    }
}
