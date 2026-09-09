<script setup lang="ts">
import type { ManagedAccount } from "../types/account";
import QuotaRing from "./QuotaRing.vue";
import QuotaRows from "./QuotaRows.vue";

defineProps<{ account: ManagedAccount; codexRunning: boolean; switching: boolean }>();
defineEmits<{ switch: [account: ManagedAccount] }>();
</script>

<template>
  <article :class="['account-row', { active: account.isActive }]">
    <div class="identity">
      <QuotaRing
        v-if="account.isActive"
        :five-hour="account.fiveHour"
        :weekly="account.weekly"
        :codex-running="codexRunning"
      />
      <span v-else class="initial">{{ (account.alias || account.email).charAt(0).toUpperCase() }}</span>
    </div>
    <div class="account-content">
      <header>
        <strong>{{ account.alias || account.email }}</strong>
        <span class="plan">{{ account.planType?.toUpperCase() || "CHATGPT" }}</span>
      </header>
      <QuotaRows :five-hour="account.fiveHour" :weekly="account.weekly" />
      <small v-if="account.lastError" :title="account.lastError">数据已过期</small>
    </div>
    <button
      :class="['action', { current: account.isActive }]"
      :disabled="account.isActive || switching"
      @click.stop="$emit('switch', account)"
    >
      {{ account.isActive ? "当前" : switching ? "切换中" : "切换" }}
    </button>
  </article>
</template>

<style scoped>
.account-row {
  min-height: 96px;
  display: grid;
  grid-template-columns: 82px minmax(0, 1fr) 68px;
  align-items: center;
  gap: 10px;
  padding: 7px 12px 7px 8px;
  border-top: 1px solid rgba(145, 161, 183, .25);
}
.identity { width: 76px; display: grid; place-items: center; }
.initial {
  width: 42px; height: 42px; border-radius: 50%; display: grid; place-items: center;
  color: #f0f5fb; font-weight: 650; font-size: 19px;
  background: linear-gradient(145deg, #303b4d, #1d2532);
  border: 1px solid #47546a;
}
.account-content { display: grid; gap: 8px; min-width: 0; }
header { display: flex; align-items: center; gap: 9px; min-width: 0; }
strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
.plan { border: 1px solid #8062e8; color: #ece7ff; background: #30225e; border-radius: 6px; padding: 2px 7px; font-size: 10px; font-weight: 750; }
.action { width: 64px; height: 32px; border-radius: 8px; border: 1px solid #1685ff; color: white; background: #087ef5; font-size: 12px; cursor: pointer; }
.action:hover:not(:disabled) { background: #2392ff; }
.action.current { border-color: #2fbf86; color: #66e5ad; background: rgba(24, 139, 94, .18); }
.action:disabled { cursor: default; opacity: .9; }
small { color: #e8a96d; font-size: 10px; position: absolute; }
</style>
