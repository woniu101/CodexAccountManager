<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import type { QuotaWindow } from "../types/account";

defineProps<{ fiveHour?: QuotaWindow; weekly?: QuotaWindow }>();

const now = ref(Date.now());
let clock: number | undefined;
onMounted(() => {
  clock = window.setInterval(() => { now.value = Date.now(); }, 60_000);
});
onBeforeUnmount(() => window.clearInterval(clock));

const percent = (quota?: QuotaWindow) => quota ? `${Math.round(quota.remainingPercent)}%` : "--";
const reset = (quota?: QuotaWindow) => {
  if (!quota?.resetsAt) return "暂无重置时间";
  const target = new Date(quota.resetsAt * 1000);
  const difference = target.getTime() - now.value;
  if (difference > 0 && difference < 24 * 60 * 60 * 1000) {
    const hours = Math.floor(difference / 3_600_000);
    const minutes = Math.max(0, Math.floor((difference % 3_600_000) / 60_000));
    return `${hours}小时${minutes}分后`;
  }
  return target.toLocaleString("zh-CN", { weekday: "short", hour: "2-digit", minute: "2-digit" });
};
</script>

<template>
  <div class="quota-rows">
    <div class="quota-row">
      <svg class="quota-icon" viewBox="0 0 20 20" aria-hidden="true">
        <circle cx="10" cy="10" r="7" />
        <path d="M10 5.8v4.5l3 1.7" />
      </svg>
      <span class="quota-label">5h 剩余 {{ percent(fiveHour) }}</span>
      <span class="bar"><i class="five" :style="{ width: percent(fiveHour) }" /></span>
      <time>{{ reset(fiveHour) }}</time>
    </div>
    <div class="quota-row">
      <svg class="quota-icon" viewBox="0 0 20 20" aria-hidden="true">
        <rect x="3" y="4.5" width="14" height="12" rx="2" />
        <path d="M6.5 3v3M13.5 3v3M3 8h14" />
      </svg>
      <span class="quota-label">本周剩余 {{ percent(weekly) }}</span>
      <span class="bar"><i class="weekly" :style="{ width: percent(weekly) }" /></span>
      <time>{{ reset(weekly) }}</time>
    </div>
  </div>
</template>

<style scoped>
.quota-rows { display: grid; gap: 6px; min-width: 0; }
.quota-row {
  display: grid;
  grid-template-columns: 16px 80px minmax(38px, 1fr) 68px;
  align-items: center;
  gap: 4px;
  min-width: 0;
  color: #dce5f2;
  font-size: 12px;
  line-height: 1;
}
.quota-icon { width: 16px; height: 16px; fill: none; stroke: #e6edf7; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.quota-label { white-space: nowrap; }
.bar { display: block; height: 6px; border-radius: 999px; background: #293647; overflow: hidden; }
.bar i { display: block; height: 100%; border-radius: inherit; transition: width .45s ease-out; }
.bar .five { background: #4dd7ff; }
.bar .weekly { background: #9a7bff; }
time { text-align: left; color: #aeb9c9; white-space: nowrap; font-size: 10px; }
</style>
