<script setup lang="ts">
import type { ManagedAccount, ProcessState } from "../types/account";
import QuotaRing from "./QuotaRing.vue";
import QuotaRows from "./QuotaRows.vue";

defineProps<{ account?: ManagedAccount; processState: ProcessState; loading: boolean }>();
defineEmits<{ expand: [] }>();
</script>

<template>
  <section class="summary pill" @click="$emit('expand')">
    <QuotaRing :five-hour="account?.fiveHour" :weekly="account?.weekly" :process-state="processState" />
    <div v-if="account" class="summary-content">
      <header>
        <strong>{{ account.alias || account.email }}</strong>
        <span class="plan">{{ account.planType?.toUpperCase() || "CHATGPT" }}</span>
      </header>
      <QuotaRows :five-hour="account.fiveHour" :weekly="account.weekly" />
    </div>
    <div v-else class="empty-summary">
      <strong>{{ loading ? "正在读取 Codex…" : "尚未导入账号" }}</strong>
      <span>点击打开账号管理器</span>
    </div>
  </section>
</template>

<style scoped>
.summary {
  width: 416px;
  height: 80px;
  padding: 4px;
  display: grid;
  grid-template-columns: 72px 1fr;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}
.pill { border-radius: 999px; }
.summary-content { min-width: 0; padding-right: 10px; display: grid; gap: 7px; }
header { display: flex; align-items: center; gap: 10px; min-width: 0; }
strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #f5f8fd; font-size: 13px; }
.plan { border: 1px solid #8565ec; background: #332264; color: #f0eaff; border-radius: 6px; padding: 2px 7px; font-size: 10px; font-weight: 750; }
.empty-summary { display: grid; gap: 4px; text-align: left; }
.empty-summary span { color: #96a4b8; font-size: 12px; }
</style>
