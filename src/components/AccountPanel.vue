<script setup lang="ts">
import { computed } from "vue";
import type { LoginProgress, ManagedAccount, ProcessState, UserSettings } from "../types/account";
import AccountRow from "./AccountRow.vue";
import SettingsView from "./SettingsView.vue";

const props = defineProps<{
  accounts: ManagedAccount[];
  processState: ProcessState;
  switchingId?: string;
  loading: boolean;
  settingsOpen: boolean;
  settings: UserSettings;
  savingSettings: boolean;
  login: LoginProgress;
  addingAccount: boolean;
}>();
const emit = defineEmits<{
  add: [];
  settings: [];
  importCurrent: [];
  switch: [account: ManagedAccount];
  updateSettings: [settings: UserSettings];
  refresh: [];
  collapse: [];
  cancelLogin: [];
}>();

const current = computed(() => props.accounts.find((account) => account.isActive));
const others = computed(() => props.accounts.filter((account) => !account.isActive));
</script>

<template>
  <section class="account-panel">
    <nav class="panel-actions">
      <button v-if="addingAccount" class="login-progress active" disabled>
        <span class="spinner" /> 正在准备授权
      </button>
      <button
        v-else-if="login.status === 'waiting'"
        class="login-progress active"
        @click.stop="emit('cancelLogin')"
      >
        <span class="spinner" /> 等待授权 <em>取消</em>
      </button>
      <button v-else :class="{ active: !settingsOpen }" @click.stop="emit('add')">
        <span class="action-icon">＋</span> 添加账号
      </button>
      <button :class="{ active: settingsOpen }" @click.stop="emit('settings')">
        <span class="action-icon gear">⚙</span> 设置
      </button>
    </nav>

    <SettingsView
      v-if="settingsOpen"
      :settings="settings"
      :saving="savingSettings"
      @update="emit('updateSettings', $event)"
      @refresh="emit('refresh')"
    />

    <div v-else-if="accounts.length" class="accounts-body">
      <div class="other-list">
        <AccountRow
          v-for="account in others"
          :key="account.id"
          :account="account"
          :process-state="processState"
          :switching="switchingId === account.id"
          @switch="emit('switch', $event)"
        />
      </div>
      <AccountRow
        v-if="current"
        class="current-row"
        :account="current"
        :process-state="processState"
        :switching="false"
        @switch="emit('switch', $event)"
        @collapse="emit('collapse')"
      />
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
.account-panel { width: 416px; height: 100%; overflow: hidden; border-radius: 32px; display: flex; flex-direction: column; }
.panel-actions { height: 52px; flex: 0 0 52px; display: grid; grid-template-columns: 1fr 1fr; }
.panel-actions button { position: relative; border: 0; color: #dce5f1; background: transparent; cursor: pointer; font-size: 13px; }
.panel-actions button + button { border-left: 1px solid rgba(145, 161, 183, .24); }
.panel-actions button:hover, .panel-actions button.active { background: rgba(255, 255, 255, .035); color: #f5f8fc; }
.panel-actions button.active::after { content: ""; position: absolute; left: 35%; right: 35%; bottom: 0; height: 2px; border-radius: 2px; background: #4dd7ff; opacity: .75; }
.action-icon { display: inline-block; margin-right: 8px; font-size: 20px; font-weight: 350; vertical-align: -1px; }
.action-icon.gear { font-size: 16px; }
.panel-actions .login-progress { color: #dce8f5; }
.panel-actions .login-progress:disabled { cursor: wait; }
.panel-actions .login-progress em { margin-left: 7px; color: #4dd7ff; font-size: 11px; font-style: normal; }
.spinner { display: inline-block; width: 12px; height: 12px; margin-right: 7px; border: 2px solid #3b526b; border-top-color: #4dd7ff; border-radius: 50%; vertical-align: -2px; animation: spin .8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.accounts-body { display: flex; flex: 1; min-height: 0; flex-direction: column; border-top: 1px solid rgba(145, 161, 183, .24); }
.other-list { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; scrollbar-width: thin; scrollbar-color: #46566c transparent; }
.current-row { flex: 0 0 auto; background: rgba(16, 23, 33, .72); border-top-color: rgba(166, 185, 211, .36); }
.empty-panel { flex: 1; border-top: 1px solid rgba(145, 161, 183, .24); display: grid; place-content: center; gap: 9px; text-align: center; color: #eef4fc; }
.empty-panel span { color: #95a2b4; font-size: 12px; }
.empty-panel button { justify-self: center; margin-top: 6px; border: 1px solid #1685ff; background: #087ef5; color: white; border-radius: 8px; padding: 8px 14px; cursor: pointer; }
</style>
