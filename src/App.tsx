import { useEffect, useMemo, useState } from "react"
import {
  Avatar,
  Badge,
  Button,
  Card,
  CardHeader,
  createTableColumn,
  DataGrid,
  DataGridBody,
  DataGridCell,
  DataGridHeader,
  DataGridHeaderCell,
  DataGridRow,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  Divider,
  Field,
  Input,
  MessageBar,
  MessageBarBody,
  Select,
  Spinner,
  Switch,
  Tab,
  TabList,
  TableCellLayout,
  TableColumnDefinition,
  Text,
  Title2,
  Title3,
  Toolbar,
  ToolbarButton,
  Title1,
} from "@fluentui/react-components"
import {
  Add20Regular,
  ArrowClockwise20Regular,
  Delete20Regular,
  Dismiss20Regular,
  Edit20Regular,
  FolderAdd20Regular,
  PersonAdd20Regular,
  Play20Regular,
  Save20Regular,
  Settings20Regular,
  Square20Regular,
  Subtract20Regular,
} from "@fluentui/react-icons"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { api } from "./api"
import type { AccountInfo, GroupInfo } from "./types"

const DEFAULT_GROUP_ID = "default"
const isTauriWindow = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

const withCurrentWindow = (
  action: (currentWindow: ReturnType<typeof getCurrentWindow>) => Promise<void>,
) => {
  if (!isTauriWindow()) return
  void action(getCurrentWindow()).catch(() => undefined)
}

export default function App() {
  const [accounts, setAccounts] = useState<AccountInfo[]>([])
  const [groups, setGroups] = useState<GroupInfo[]>([])
  const [selectedGroupId, setSelectedGroupId] = useState(DEFAULT_GROUP_ID)
  const [saveGroupId, setSaveGroupId] = useState(DEFAULT_GROUP_ID)
  const [remark, setRemark] = useState("")
  const [busy, setBusy] = useState(false)
  const [panelOpen, setPanelOpen] = useState(false)
  const [notice, setNotice] = useState("")
  const [error, setErrorText] = useState("")
  const [autoStart, setAutoStart] = useState(false)
  const [editingAccountId, setEditingAccountId] = useState<string | null>(null)

  const orderedGroups = useMemo(() => {
    const next = [...groups]
    if (!next.some((group) => group.Id === DEFAULT_GROUP_ID)) {
      next.unshift({
        Id: DEFAULT_GROUP_ID,
        Name: "默认分组",
        CreatedAt: "0001-01-01T00:00:00",
      })
    }

    return next.sort((a, b) => {
      if (a.Id === DEFAULT_GROUP_ID) return -1
      if (b.Id === DEFAULT_GROUP_ID) return 1
      return new Date(a.CreatedAt).getTime() - new Date(b.CreatedAt).getTime()
    })
  }, [groups])

  const sortedAccounts = useMemo(() => {
    return [...accounts].sort(
      (a, b) => new Date(b.LastUsed).getTime() - new Date(a.LastUsed).getTime(),
    )
  }, [accounts])

  const visibleAccounts = useMemo(() => {
    return sortedAccounts.filter(
      (account) => account.GroupId === selectedGroupId,
    )
  }, [selectedGroupId, sortedAccounts])

  const selectedGroup = useMemo(() => {
    return (
      orderedGroups.find((group) => group.Id === selectedGroupId) ??
      orderedGroups[0]
    )
  }, [orderedGroups, selectedGroupId])

  const activeAccount = sortedAccounts[0] ?? null
  const groupCounts = useMemo(() => {
    return accounts.reduce<Record<string, number>>((result, account) => {
      result[account.GroupId] = (result[account.GroupId] ?? 0) + 1
      return result
    }, {})
  }, [accounts])

  const columns = useMemo<TableColumnDefinition<AccountInfo>[]>(
    () => [
      createTableColumn<AccountInfo>({
        columnId: "account",
        renderHeaderCell: () => "账号",
        renderCell: (account) => (
          <TableCellLayout
            media={<Avatar name={account.Remark || "账号"} color="brand" />}
          >
            <span className="cell-title">
              <span>{account.Remark}</span>
              {account.LoggedIn && (
                <Badge color="success">已登录</Badge>
              )}
              {activeAccount?.Id === account.Id && (
                <Badge color="brand">最近</Badge>
              )}
            </span>
          </TableCellLayout>
        ),
      }),
      createTableColumn<AccountInfo>({
        columnId: "lastUsed",
        renderHeaderCell: () => "上次使用",
        renderCell: (account) => formatDate(account.LastUsed),
      }),
      createTableColumn<AccountInfo>({
        columnId: "actions",
        renderHeaderCell: () => "操作",
        renderCell: (account) => (
          <div className="table-actions">
            <Button
              size="small"
              icon={<Edit20Regular />}
              onClick={() => openEditDialog(account)}
            >
              编辑
            </Button>
            <Button
              size="small"
              icon={<Delete20Regular />}
              onClick={() => deleteAccount(account)}
            >
              删除
            </Button>
            <Button
              size="small"
              appearance="primary"
              icon={<Play20Regular />}
              onClick={() => switchAccount(account)}
              disabled={busy}
            >
              切换
            </Button>
          </div>
        ),
      }),
    ],
    [activeAccount?.Id, busy],
  )

  const showMessage = (message: string) => {
    setNotice(message)
    setErrorText("")
  }

  const showError = (message: string) => {
    setErrorText(message)
    setNotice("")
  }

  const loadData = async () => {
    setBusy(true)
    try {
      const [nextGroups, nextAccounts, nextAutoStart] = await Promise.all([
        api.getGroups(),
        api.getAccounts(),
        api.getAutoStart(),
      ])

      setGroups(nextGroups)
      setAccounts(nextAccounts)
      setAutoStart(nextAutoStart)

      const validGroupIds = new Set(nextGroups.map((group) => group.Id))
      validGroupIds.add(DEFAULT_GROUP_ID)
      if (!validGroupIds.has(selectedGroupId)) {
        setSelectedGroupId(DEFAULT_GROUP_ID)
      }
    } catch {
      showError("读取配置失败。")
    } finally {
      setBusy(false)
    }
  }

  useEffect(() => {
    void loadData()
  }, [])

  useEffect(() => {
    if (!notice && !error) return

    const timer = window.setTimeout(() => {
      setNotice("")
      setErrorText("")
    }, 3000)

    return () => window.clearTimeout(timer)
  }, [notice, error])

  const openSaveDialog = () => {
    setEditingAccountId(null)
    setRemark("")
    setSaveGroupId(selectedGroupId)
    setPanelOpen(true)
  }

  const openEditDialog = (account: AccountInfo) => {
    setEditingAccountId(account.Id)
    setRemark(account.Remark)
    setSaveGroupId(account.GroupId || DEFAULT_GROUP_ID)
    setPanelOpen(true)
  }

  const closeDialog = () => {
    setPanelOpen(false)
  }

  const saveAccount = async () => {
    setBusy(true)
    try {
      const targetGroupId = orderedGroups.some(
        (group) => group.Id === saveGroupId,
      )
        ? saveGroupId
        : DEFAULT_GROUP_ID

      if (editingAccountId) {
        const updated = await api.updateAccountInfo(editingAccountId, remark)
        const moved = updated
          ? await api.moveAccountToGroup(editingAccountId, targetGroupId)
          : false
        if (!updated || !moved) {
          showError("保存账号信息失败。")
          return
        }
        showMessage("账号信息已更新。")
      } else {
        const saved = await api.saveCurrentAccountToGroup(remark, targetGroupId)
        if (!saved) {
          showError("没有找到当前战网登录配置，无法保存。")
          return
        }
        showMessage("当前登录状态已保存。")
      }

      setSelectedGroupId(targetGroupId)
      closeDialog()
      await loadData()
    } catch {
      showError("保存失败。")
    } finally {
      setBusy(false)
    }
  }

  const createGroup = async () => {
    const name = window.prompt("输入新分组名称")
    if (!name?.trim()) return

    const group = await api.createGroup(name.trim())
    if (!group) {
      showError("创建分组失败。")
      return
    }

    setSelectedGroupId(group.Id)
    setSaveGroupId(group.Id)
    showMessage("分组已创建。")
    await loadData()
  }

  const renameGroup = async () => {
    if (!selectedGroup || selectedGroup.Id === DEFAULT_GROUP_ID) return

    const name = window.prompt("输入新的分组名称", selectedGroup.Name)
    if (!name?.trim()) return

    const ok = await api.renameGroup(selectedGroup.Id, name.trim())
    ok
      ? showMessage("分组已重命名。")
      : showError("重命名失败，可能存在同名分组。")
    await loadData()
  }

  const deleteGroup = async () => {
    if (!selectedGroup || selectedGroup.Id === DEFAULT_GROUP_ID) return
    if (
      !window.confirm(
        `删除分组“${selectedGroup.Name}”？组内账号会移动到默认分组。`,
      )
    )
      return

    const ok = await api.deleteGroup(selectedGroup.Id)
    if (ok) {
      setSelectedGroupId(DEFAULT_GROUP_ID)
      showMessage("分组已删除。")
    } else {
      showError("删除分组失败。")
    }
    await loadData()
  }

  const switchAccount = async (account: AccountInfo) => {
    setBusy(true)
    try {
      const ok = await api.switchAccount(account.Id)
      if (!ok) {
        showError("切换失败，请确认该账号配置仍然存在。")
        return
      }
      showMessage(`已切换到 ${account.Remark}。`)
      await loadData()
    } catch {
      showError("切换失败。")
    } finally {
      setBusy(false)
    }
  }

  const deleteAccount = async (account: AccountInfo) => {
    if (!window.confirm(`删除账号“${account.Remark}”？`)) return

    const ok = await api.deleteAccount(account.Id)
    ok ? showMessage("账号记录已删除。") : showError("删除账号失败。")
    await loadData()
  }

  const loginNewAccount = async () => {
    const ok = await api.addNewAccount()
    ok
      ? showMessage("已清空当前登录配置并启动战网。")
      : showError("启动登录流程失败。")
  }

  const toggleAutoStart = async (_: unknown, data: { checked: boolean }) => {
    const previous = autoStart
    setAutoStart(data.checked)
    const ok = await api.setAutoStart(data.checked)
    if (!ok) {
      setAutoStart(previous)
      showError("更新开机启动失败。")
    }
  }

  const minimizeWindow = () => {
    withCurrentWindow((currentWindow) => currentWindow.minimize())
  }

  const toggleMaximizeWindow = () => {
    withCurrentWindow((currentWindow) => currentWindow.toggleMaximize())
  }

  const closeWindow = () => {
    withCurrentWindow((currentWindow) => currentWindow.close())
  }

  const formatDate = (value: string) => {
    if (!value) return "未使用"
    return new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(value))
  }

  return (
    <div className="app-frame">
      <header className="window-titlebar" data-tauri-drag-region>
        <div className="window-titlebar-brand" data-tauri-drag-region></div>
        <div className="window-titlebar-drag" data-tauri-drag-region />
        <div className="window-controls">
          <button
            className="window-control-button"
            type="button"
            aria-label="最小化"
            onClick={minimizeWindow}
          >
            <Subtract20Regular />
          </button>
          <button
            className="window-control-button"
            type="button"
            aria-label="最大化"
            onClick={toggleMaximizeWindow}
          >
            <Square20Regular />
          </button>
          <button
            className="window-control-button window-control-close"
            type="button"
            aria-label="关闭"
            onClick={closeWindow}
          >
            <Dismiss20Regular />
          </button>
        </div>
      </header>

      <main className="app-shell">
        <aside className="app-sidebar">
          <Card appearance="subtle">
            <CardHeader
              header={<Title2>战网账号切换</Title2>}
              description={<Text size={200}>账号配置切换与分组管理</Text>}
            />
          </Card>

          <div>
            <Divider />
          </div>

          <TabList
            vertical
            selectedValue={selectedGroupId}
            onTabSelect={(_, data) => setSelectedGroupId(String(data.value))}
          >
            {orderedGroups.map((group) => (
              <Tab key={group.Id} value={group.Id}>
                <span className="tab-label">
                  <span>{group.Name}</span>
                  <Badge appearance="tint">{groupCounts[group.Id] ?? 0}</Badge>
                </span>
              </Tab>
            ))}
          </TabList>

          <div className="sidebar-footer">
            <Button icon={<FolderAdd20Regular />} onClick={createGroup}>
              新建分组
            </Button>
            <Switch
              checked={autoStart}
              onChange={toggleAutoStart}
              label="开机启动"
            />
          </div>
        </aside>

        <section className="content-pane">
          <header className="content-header">
            <div>
              <Title3>{selectedGroup?.Name || "默认分组"}</Title3>
            </div>

            <Toolbar aria-label="账号操作">
              <ToolbarButton
                icon={<ArrowClockwise20Regular />}
                onClick={loadData}
                disabled={busy}
              >
                刷新
              </ToolbarButton>
              <ToolbarButton
                icon={<PersonAdd20Regular />}
                onClick={loginNewAccount}
              >
                登录新号
              </ToolbarButton>
              <ToolbarButton
                icon={<Add20Regular />}
                appearance="primary"
                onClick={openSaveDialog}
              >
                保存当前
              </ToolbarButton>
            </Toolbar>
          </header>

          {(notice || error) && (
            <MessageBar intent={error ? "error" : "success"}>
              <MessageBarBody>{notice || error}</MessageBarBody>
            </MessageBar>
          )}

          {selectedGroup?.Id !== DEFAULT_GROUP_ID && (
            <div className="group-command-row">
              <Button icon={<Edit20Regular />} onClick={renameGroup}>
                重命名分组
              </Button>
              <Button icon={<Delete20Regular />} onClick={deleteGroup}>
                删除分组
              </Button>
            </div>
          )}

          <section className="account-section" aria-busy={busy}>
            {busy && (
              <div className="loading-row">
                <Spinner size="tiny" label="正在处理" />
              </div>
            )}

            {visibleAccounts.length === 0 ? (
              <Card className="empty-card">
                <Settings20Regular className="empty-icon" />
                <Text size={500}>此分组没有账号</Text>
                <Text>
                  在浏览器开发模式下可以直接保存模拟账号；在 Windows Tauri
                  应用中会保存真实 Battle.net 登录配置。
                </Text>
                <Button
                  appearance="primary"
                  icon={<Add20Regular />}
                  onClick={openSaveDialog}
                >
                  保存当前状态
                </Button>
              </Card>
            ) : (
              <DataGrid
                items={visibleAccounts}
                columns={columns}
                getRowId={(account) => account.Id}
                focusMode="composite"
                sortable
              >
                <DataGridHeader>
                  <DataGridRow>
                    {({ renderHeaderCell }) => (
                      <DataGridHeaderCell>
                        {renderHeaderCell()}
                      </DataGridHeaderCell>
                    )}
                  </DataGridRow>
                </DataGridHeader>
                <DataGridBody<AccountInfo>>
                  {({ item, rowId }) => (
                    <DataGridRow<AccountInfo> key={rowId}>
                      {({ renderCell }) => (
                        <DataGridCell>{renderCell(item)}</DataGridCell>
                      )}
                    </DataGridRow>
                  )}
                </DataGridBody>
              </DataGrid>
            )}
          </section>
        </section>

        <Dialog
          open={panelOpen}
          onOpenChange={(_, data) => setPanelOpen(data.open)}
        >
          <DialogSurface>
            <DialogBody>
              <DialogTitle
                action={
                  <Button
                    appearance="subtle"
                    aria-label="关闭"
                    icon={<Dismiss20Regular />}
                    onClick={closeDialog}
                  />
                }
              >
                {editingAccountId ? "编辑账号" : "保存当前登录状态"}
              </DialogTitle>

              <DialogContent className="dialog-form">
                <Field label="账号备注" required>
                  <Input
                    value={remark}
                    onChange={(_, data) => setRemark(data.value)}
                    placeholder="例如：主力账号"
                    autoFocus
                  />
                </Field>
                <Field label="分组">
                  <Select
                    value={saveGroupId}
                    onChange={(event) =>
                      setSaveGroupId(event.currentTarget.value)
                    }
                  >
                    {orderedGroups.map((group) => (
                      <option key={group.Id} value={group.Id}>
                        {group.Name}
                      </option>
                    ))}
                  </Select>
                </Field>
              </DialogContent>

              <DialogActions>
                <Button onClick={closeDialog}>取消</Button>
                <Button
                  appearance="primary"
                  icon={<Save20Regular />}
                  onClick={saveAccount}
                  disabled={busy}
                >
                  {editingAccountId ? "保存修改" : "保存配置"}
                </Button>
              </DialogActions>
            </DialogBody>
          </DialogSurface>
        </Dialog>
      </main>
    </div>
  )
}
