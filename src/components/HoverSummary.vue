<script setup lang="ts">
import type { ManagedAccount, WindowPlacement } from "../types/account";
import { planLabel } from "../utils/plan";
import QuotaRows from "./QuotaRows.vue";

defineProps<{
  account?: ManagedAccount;
  loading: boolean;
  horizontal: WindowPlacement["horizontal"];
}>();
</script>

<template>
  <section :class="['summary', `opens-${horizontal}`]">
    <div v-if="account" class="summary-content">
      <header>
        <strong>{{ account.alias || account.email }}</strong>
        <span class="plan">{{ planLabel(account.planType) }}</span>
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
  padding: 10px 36px 10px 80px;
  display: flex;
  align-items: center;
  cursor: pointer;
}
.summary.opens-left { padding: 10px 80px 10px 36px; }
.summary-content { width: 100%; min-width: 0; display: grid; gap: 7px; }
header { display: flex; align-items: center; gap: 6px; min-width: 0; }
strong { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #f5f8fd; font-size: 13px; }
.plan { flex: 0 0 auto; border: 1px solid #8565ec; background: #332264; color: #f0eaff; border-radius: 6px; padding: 2px 7px; font-size: 10px; font-weight: 750; }
.empty-summary { display: grid; gap: 4px; text-align: left; }
.empty-summary span { color: #96a4b8; font-size: 12px; }
</style>
