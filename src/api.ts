import { invoke } from '@tauri-apps/api/core';
import type { AccountInfo, GroupInfo } from './types';

const DEFAULT_GROUP_ID = 'default';
const DEV_STATE_KEY = 'storm-switch-dev-state';

interface DevState {
  accounts: AccountInfo[];
  groups: GroupInfo[];
  autoStart: boolean;
}

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const now = () => new Date().toISOString();

const defaultGroup = (): GroupInfo => ({
  Id: DEFAULT_GROUP_ID,
  Name: '默认分组',
  CreatedAt: '0001-01-01T00:00:00',
});

const normalizeState = (state: Partial<DevState>): DevState => {
  const groups = state.groups?.length ? state.groups : [defaultGroup()];
  if (!groups.some((group) => group.Id === DEFAULT_GROUP_ID)) {
    groups.unshift(defaultGroup());
  }

  const groupIds = new Set(groups.map((group) => group.Id));
  const accounts = (state.accounts ?? []).map((account) => ({
    ...account,
    Remark: account.Remark?.trim() || '未命名账号',
    Username: account.Username ?? '',
    LastUsed: account.LastUsed || now(),
    GroupId: groupIds.has(account.GroupId) ? account.GroupId : DEFAULT_GROUP_ID,
  }));

  return {
    accounts,
    groups,
    autoStart: Boolean(state.autoStart),
  };
};

class DevApi {
  private read(): DevState {
    try {
      return normalizeState(JSON.parse(localStorage.getItem(DEV_STATE_KEY) || '{}'));
    } catch {
      return normalizeState({});
    }
  }

  private write(state: DevState) {
    localStorage.setItem(DEV_STATE_KEY, JSON.stringify(normalizeState(state)));
  }

  async getAccounts() {
    return this.read().accounts;
  }

  async getGroups() {
    return this.read().groups;
  }

  async createGroup(name: string) {
    const trimmed = name.trim();
    if (!trimmed) return null;

    const state = this.read();
    const existing = state.groups.find((group) => group.Name.toLowerCase() === trimmed.toLowerCase());
    if (existing) return existing;

    const group: GroupInfo = {
      Id: crypto.randomUUID(),
      Name: trimmed,
      CreatedAt: now(),
    };
    state.groups.push(group);
    this.write(state);
    return group;
  }

  async renameGroup(id: string, name: string) {
    if (id === DEFAULT_GROUP_ID) return false;

    const trimmed = name.trim();
    if (!trimmed) return false;

    const state = this.read();
    if (state.groups.some((group) => group.Id !== id && group.Name.toLowerCase() === trimmed.toLowerCase())) {
      return false;
    }

    const group = state.groups.find((item) => item.Id === id);
    if (!group) return false;

    group.Name = trimmed;
    this.write(state);
    return true;
  }

  async deleteGroup(id: string) {
    if (id === DEFAULT_GROUP_ID) return false;

    const state = this.read();
    const before = state.groups.length;
    state.groups = state.groups.filter((group) => group.Id !== id);
    if (state.groups.length === before) return false;

    state.accounts = state.accounts.map((account) => (
      account.GroupId === id ? { ...account, GroupId: DEFAULT_GROUP_ID } : account
    ));
    this.write(state);
    return true;
  }

  async moveAccountToGroup(accountId: string, groupId: string) {
    const state = this.read();
    const account = state.accounts.find((item) => item.Id === accountId);
    if (!account) return false;

    account.GroupId = state.groups.some((group) => group.Id === groupId) ? groupId : DEFAULT_GROUP_ID;
    this.write(state);
    return true;
  }

  async updateAccountInfo(accountId: string, remark: string) {
    const state = this.read();
    const account = state.accounts.find((item) => item.Id === accountId);
    if (!account) return false;

    account.Remark = remark.trim() || '未命名账号';
    this.write(state);
    return true;
  }

  async saveCurrentAccountToGroup(remark: string, groupId: string) {
    const state = this.read();
    const targetGroupId = state.groups.some((group) => group.Id === groupId) ? groupId : DEFAULT_GROUP_ID;
    state.accounts.push({
      Id: crypto.randomUUID(),
      Remark: remark.trim() || '未命名账号',
      Username: '',
      LastUsed: now(),
      GroupId: targetGroupId,
    });
    this.write(state);
    return true;
  }

  async switchAccount(id: string) {
    const state = this.read();
    const account = state.accounts.find((item) => item.Id === id);
    if (!account) return false;

    account.LastUsed = now();
    this.write(state);
    return true;
  }

  async deleteAccount(id: string) {
    const state = this.read();
    const before = state.accounts.length;
    state.accounts = state.accounts.filter((account) => account.Id !== id);
    this.write(state);
    return state.accounts.length !== before;
  }

  async addNewAccount() {
    return true;
  }

  async getAutoStart() {
    return this.read().autoStart;
  }

  async setAutoStart(enabled: boolean) {
    const state = this.read();
    state.autoStart = enabled;
    this.write(state);
    return true;
  }
}

const devApi = new DevApi();

export const api = {
  getAccounts: () => isTauri() ? invoke<AccountInfo[]>('get_accounts') : devApi.getAccounts(),
  getGroups: () => isTauri() ? invoke<GroupInfo[]>('get_groups') : devApi.getGroups(),
  createGroup: (name: string) => isTauri() ? invoke<GroupInfo | null>('create_group', { name }) : devApi.createGroup(name),
  renameGroup: (id: string, name: string) => isTauri() ? invoke<boolean>('rename_group', { id, name }) : devApi.renameGroup(id, name),
  deleteGroup: (id: string) => isTauri() ? invoke<boolean>('delete_group', { id }) : devApi.deleteGroup(id),
  moveAccountToGroup: (accountId: string, groupId: string) =>
    isTauri() ? invoke<boolean>('move_account_to_group', { accountId, groupId }) : devApi.moveAccountToGroup(accountId, groupId),
  updateAccountInfo: (accountId: string, remark: string) =>
    isTauri() ? invoke<boolean>('update_account_info', { accountId, remark }) : devApi.updateAccountInfo(accountId, remark),
  saveCurrentAccountToGroup: (remark: string, groupId: string) =>
    isTauri() ? invoke<boolean>('save_current_account_to_group', { remark, groupId }) : devApi.saveCurrentAccountToGroup(remark, groupId),
  switchAccount: (id: string) => isTauri() ? invoke<boolean>('switch_account', { id }) : devApi.switchAccount(id),
  deleteAccount: (id: string) => isTauri() ? invoke<boolean>('delete_account', { id }) : devApi.deleteAccount(id),
  addNewAccount: () => isTauri() ? invoke<boolean>('add_new_account') : devApi.addNewAccount(),
  getAutoStart: () => isTauri() ? invoke<boolean>('get_auto_start') : devApi.getAutoStart(),
  setAutoStart: (enabled: boolean) => isTauri() ? invoke<boolean>('set_auto_start', { enabled }) : devApi.setAutoStart(enabled),
};
