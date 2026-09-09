<script setup lang="ts">
import type { QuotaWindow } from "../types/account";

defineProps<{ fiveHour?: QuotaWindow; weekly?: QuotaWindow }>();

const percent = (quota?: QuotaWindow) => quota ? `${Math.round(quota.remainingPercent)}%` : "--";
const reset = (quota?: QuotaWindow) => {
  if (!quota?.resetsAt) return "暂无重置时间";
  const target = new Date(quota.resetsAt * 1000);
  const difference = target.getTime() - Date.now();
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
      <span class="quota-icon">◷</span>
      <span class="quota-label">5h 剩余 {{ percent(fiveHour) }}</span>
      <span class="bar"><i class="five" :style="{ width: percent(fiveHour) }" /></span>
      <time>{{ reset(fiveHour) }}</time>
    </div>
    <div class="quota-row">
      <span class="quota-icon calendar">□</span>
      <span class="quota-label">本周剩余 {{ percent(weekly) }}</span>
      <span class="bar"><i class="weekly" :style="{ width: percent(weekly) }" /></span>
      <time>{{ reset(weekly) }}</time>
    </div>
  </div>
</template>

<style scoped>
.quota-rows { display: grid; gap: 7px; min-width: 0; }
.quota-row {
  display: grid;
  grid-template-columns: 18px 98px minmax(70px, 1fr) 94px;
  align-items: center;
  gap: 7px;
  min-width: 0;
  color: #dce5f2;
  font-size: 12px;
  line-height: 1;
}
.quota-icon { color: #e6edf7; font-size: 18px; text-align: center; }
.quota-icon.calendar { font-size: 14px; border: 1px solid #dce5f2; border-radius: 3px; height: 13px; line-height: 9px; }
.quota-label { white-space: nowrap; }
.bar { display: block; height: 6px; border-radius: 999px; background: #293647; overflow: hidden; }
.bar i { display: block; height: 100%; border-radius: inherit; transition: width .45s ease-out; }
.bar .five { background: #4dd7ff; }
.bar .weekly { background: #9a7bff; }
time { text-align: right; color: #aeb9c9; white-space: nowrap; font-size: 11px; }
</style>
