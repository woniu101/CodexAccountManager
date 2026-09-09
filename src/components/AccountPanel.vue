<script setup lang="ts">
import { computed } from "vue";
import type { ManagedAccount } from "../types/account";
import AccountRow from "./AccountRow.vue";

const props = defineProps<{
  accounts: ManagedAccount[];
  codexRunning: boolean;
  switchingId?: string;
  loading: boolean;
}>();
const emit = defineEmits<{
  add: [];
  settings: [];
  importCurrent: [];
  switch: [account: ManagedAccount];
}>();

const ordered = computed(() => [
  ...props.accounts.filter((account) => !account.isActive),
  ...props.accounts.filter((account) => account.isActive),
]);
</script>

<template>
  <section class="account-panel">
    <nav class="panel-actions">
      <button @click.stop="emit('add')"><b>＋</b> 添加账号</button>
      <button @click.stop="emit('settings')"><b>⚙</b> 设置</button>
    </nav>
    <div v-if="accounts.length" class="account-list">
      <AccountRow
        v-for="account in ordered"
        :key="account.id"
        :account="account"
        :codex-running="codexRunning"
        :switching="switchingId === account.id"
        @switch="emit('switch', $event)"
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
.account-panel {
  width: 428px;
  max-height: 368px;
  overflow: hidden;
  border-radius: 20px 20px 38px 38px;
  display: flex;
  flex-direction: column;
}
.panel-actions { height: 54px; flex: 0 0 54px; display: grid; grid-template-columns: 1fr 1fr; }
.panel-actions button { border: 0; color: #e4ebf5; background: transparent; cursor: pointer; font-size: 13px; }
.panel-actions button + button { border-left: 1px solid rgba(145, 161, 183, .25); }
.panel-actions button:hover { background: rgba(255, 255, 255, .05); }
.panel-actions b { font-size: 19px; margin-right: 6px; font-weight: 400; }
.account-list { overflow-y: auto; overscroll-behavior: contain; }
.account-list :deep(.account-row.active) { position: sticky; bottom: 0; background: rgba(19, 25, 35, .96); }
.empty-panel { min-height: 210px; border-top: 1px solid rgba(145, 161, 183, .25); display: grid; place-content: center; gap: 9px; text-align: center; color: #eef4fc; }
.empty-panel span { color: #95a2b4; font-size: 12px; }
.empty-panel button { justify-self: center; margin-top: 6px; border: 1px solid #1685ff; background: #087ef5; color: white; border-radius: 8px; padding: 8px 14px; cursor: pointer; }
</style>
