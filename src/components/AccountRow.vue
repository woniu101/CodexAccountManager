<script setup lang="ts">
import { computed } from "vue";
import type { ManagedAccount, ProcessState } from "../types/account";
import QuotaRing from "./QuotaRing.vue";
import QuotaRows from "./QuotaRows.vue";

const props = defineProps<{ account: ManagedAccount; processState: ProcessState; switching: boolean }>();
const emit = defineEmits<{ switch: [account: ManagedAccount]; collapse: [] }>();

const freshness = computed(() => {
  if (!props.account.lastError) return undefined;
  if (props.account.credentialState === "missing") return "凭据缺失，请重新登录";
  if (props.account.credentialState === "expired") return "认证可能已失效";
  if (!props.account.lastUpdatedAt) return "当前无法更新";
  const minutes = Math.max(1, Math.floor((Date.now() / 1000 - props.account.lastUpdatedAt) / 60));
  return `离线 · 更新于 ${minutes} 分钟前`;
});
</script>

<template>
  <article :class="['account-row', { active: account.isActive }]">
    <div class="identity" :title="account.isActive ? '点击圆球收起面板' : undefined" @click="account.isActive && emit('collapse')">
      <QuotaRing
        v-if="account.isActive"
        :five-hour="account.fiveHour"
        :weekly="account.weekly"
        :process-state="processState"
        interaction-hint="点击圆球收起面板"
      />
      <span v-else class="initial">{{ (account.alias || account.email).charAt(0).toUpperCase() }}</span>
    </div>
    <div class="account-content">
      <header>
        <strong>{{ account.alias || account.email }}</strong>
        <span class="plan">{{ account.planType?.toUpperCase() || "CHATGPT" }}</span>
      </header>
      <QuotaRows :five-hour="account.fiveHour" :weekly="account.weekly" />
      <small v-if="freshness" :title="account.lastError">{{ freshness }}</small>
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
  min-height: 102px;
  display: grid;
  grid-template-columns: 82px minmax(0, 1fr) 68px;
  align-items: center;
  gap: 10px;
  padding: 7px 12px 7px 8px;
  border-top: 1px solid rgba(145, 161, 183, .25);
}
.identity { width: 76px; display: grid; place-items: center; }
.account-row.active .identity { cursor: pointer; }
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
small { color: #e8a96d; font-size: 10px; }
</style>
