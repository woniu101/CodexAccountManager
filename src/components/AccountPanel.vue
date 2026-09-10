<script setup lang="ts">
import { computed } from "vue";
import type { LoginProgress, ManagedAccount, UserSettings, WindowPlacement } from "../types/account";
import AccountRow from "./AccountRow.vue";
import SettingsView from "./SettingsView.vue";

const props = defineProps<{
  accounts: ManagedAccount[];
  switchingId?: string;
  loading: boolean;
  settingsOpen: boolean;
  settings: UserSettings;
  savingSettings: boolean;
  settingsSaved: boolean;
  refreshStatus?: string;
  statusMessage?: string;
  statusTone?: "error" | "switching" | "info";
  login: LoginProgress;
  addingAccount: boolean;
  cancellingLogin: boolean;
  horizontal: WindowPlacement["horizontal"];
  vertical: WindowPlacement["vertical"];
}>();
const emit = defineEmits<{
  add: [];
  settings: [];
  importCurrent: [];
  switch: [account: ManagedAccount];
  remove: [account: ManagedAccount];
  updateSettings: [settings: UserSettings];
  refresh: [];
  cancelLogin: [];
  confirmDuplicate: [overwrite: boolean];
}>();

const others = computed(() => props.accounts.filter((account) => !account.isActive));
</script>

<template>
  <section :class="['account-panel', `opens-${horizontal}`, `opens-${vertical}`]">
    <nav class="panel-actions">
      <button v-if="cancellingLogin" class="login-progress active" disabled>
        <span class="spinner" /> 正在取消授权
      </button>
      <button v-else-if="addingAccount" class="login-progress active" disabled>
        <span class="spinner" /> 正在准备授权
      </button>
      <button
        v-else-if="login.status === 'waiting'"
        class="login-progress active"
        title="授权等待最多 5 分钟"
        @click.stop="emit('cancelLogin')"
      >
        <span class="spinner" /> 等待授权 <em>取消</em>
      </button>
      <button v-else :class="{ active: !settingsOpen }" @click.stop="emit('add')">
        <span class="action-icon">＋</span> 添加账号
      </button>
      <button :class="{ active: settingsOpen }" @click.stop="emit('settings')">
        <span class="action-icon gear">⚙</span> 设置
        <em v-if="settingsSaved" class="saved-label">已保存</em>
      </button>
    </nav>

    <div
      v-if="statusMessage || login.status === 'duplicate' || login.status === 'failed'"
      :class="['panel-notice', statusTone || (login.status === 'failed' ? 'error' : 'info')]"
      :title="statusMessage"
    >
      <template v-if="login.status === 'duplicate'">
        <span>账号已存在，是否更新凭据？</span>
        <button @click.stop="emit('confirmDuplicate', true)">更新</button>
        <button @click.stop="emit('confirmDuplicate', false)">取消</button>
      </template>
      <span v-else>{{ login.status === 'failed' ? login.message : statusMessage }}</span>
    </div>

    <SettingsView
      v-if="settingsOpen"
      class="settings-slot"
      :settings="settings"
      :saving="savingSettings"
      :refreshing="loading"
      :refresh-status="refreshStatus"
      @update="emit('updateSettings', $event)"
      @refresh="emit('refresh')"
    />

    <div v-else-if="accounts.length" class="accounts-body">
      <div :class="['other-list', { scrollable: others.length > 2 }]">
        <AccountRow
          v-for="account in others"
          :key="account.id"
          :account="account"
          :switching="switchingId === account.id"
          :horizontal="horizontal"
          @switch="emit('switch', $event)"
          @remove="emit('remove', $event)"
        />
      </div>
    </div>

    <div v-else class="empty-panel">
      <strong>还没有已管理的账号</strong>
      <span>导入当前 Codex 登录状态即可开始</span>
      <button :disabled="loading" @click.stop="emit('importCurrent')">
        {{ loading ? "正在导入…" : "导入当前账号" }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.account-panel { width: 416px; height: 100%; padding-bottom: 80px; overflow: hidden; border-radius: 40px; display: flex; flex-direction: column; }
.account-panel.opens-down { padding-top: 80px; padding-bottom: 0; }
.panel-actions { height: 52px; flex: 0 0 52px; display: grid; grid-template-columns: 1fr 1fr; }
.account-panel.opens-down .panel-actions { order: 3; border-top: 1px solid rgba(145, 161, 183, .24); }
.account-panel.opens-down .accounts-body,
.account-panel.opens-down .settings-slot,
.account-panel.opens-down .empty-panel { order: 1; }
.account-panel.opens-down .panel-notice { order: 2; }
.panel-actions button { position: relative; border: 0; color: #dce5f1; background: transparent; cursor: pointer; font-size: 13px; }
.panel-actions button + button { border-left: 1px solid rgba(145, 161, 183, .24); }
.panel-actions button:hover, .panel-actions button.active { background: rgba(255, 255, 255, .035); color: #f5f8fc; }
.panel-actions button.active::after { content: ""; position: absolute; left: 21%; right: 21%; bottom: 0; height: 2px; border-radius: 2px; background: #4dd7ff; opacity: .8; transition: left .2s ease, right .2s ease; }
.action-icon { display: inline-block; margin-right: 8px; font-size: 20px; font-weight: 350; vertical-align: -1px; }
.action-icon.gear { font-size: 16px; }
.panel-actions .login-progress { color: #dce8f5; }
.panel-actions .login-progress:disabled { cursor: wait; }
.panel-actions .login-progress em { margin-left: 7px; color: #4dd7ff; font-size: 11px; font-style: normal; }
.saved-label { margin-left: 6px; color: #62dfad; font-size: 9px; font-style: normal; }
.spinner { display: inline-block; width: 12px; height: 12px; margin-right: 7px; border: 2px solid #3b526b; border-top-color: #4dd7ff; border-radius: 50%; vertical-align: -2px; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.panel-notice { height: 32px; min-height: 32px; padding: 0 12px; display: flex; align-items: center; justify-content: center; gap: 6px; border-top: 1px solid rgba(145, 161, 183, .18); border-bottom: 1px solid rgba(145, 161, 183, .18); color: #dce7f6; background: rgba(30, 41, 55, .72); font-size: 10px; white-space: nowrap; overflow: hidden; }
.panel-notice span { overflow: hidden; text-overflow: ellipsis; }
.panel-notice.error { color: #ffc7a3; background: rgba(93, 44, 36, .2); }
.panel-notice.switching { color: #ffe1a0; background: rgba(102, 78, 24, .18); }
.panel-notice button { flex: 0 0 auto; padding: 2px 5px; border: 0; color: #69c5ff; background: transparent; cursor: pointer; }
.accounts-body { position: relative; display: flex; flex: 1; min-height: 0; flex-direction: column; }
.accounts-body::before { content: ""; position: absolute; z-index: 2; left: 10px; right: 10px; top: 0; height: 1px; background: rgba(145, 161, 183, .24); pointer-events: none; }
.account-panel.opens-down .accounts-body::before,
.account-panel.opens-down .other-list .account-row:first-child::before { display: none; }
.other-list { flex: 1; min-height: 0; overflow: hidden; overscroll-behavior: contain; }
.other-list.scrollable { overflow-y: auto; scrollbar-width: thin; scrollbar-color: #46566c transparent; }
.empty-panel { flex: 1; border-top: 1px solid rgba(145, 161, 183, .24); display: grid; place-content: center; gap: 9px; text-align: center; color: #eef4fc; }
.empty-panel span { color: #95a2b4; font-size: 12px; }
.empty-panel button { justify-self: center; margin-top: 6px; border: 1px solid #1685ff; background: #087ef5; color: white; border-radius: 8px; padding: 8px 14px; cursor: pointer; }
</style>
