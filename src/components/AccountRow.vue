<script setup lang="ts">
import { ref } from "vue";
import type { ManagedAccount, WindowPlacement } from "../types/account";
import { planLabel } from "../utils/plan";
import QuotaRows from "./QuotaRows.vue";

defineProps<{
  account: ManagedAccount;
  switching: boolean;
  horizontal: WindowPlacement["horizontal"];
}>();
const emit = defineEmits<{
  switch: [account: ManagedAccount];
  remove: [account: ManagedAccount];
}>();
const menuOpen = ref(false);
</script>

<template>
  <article :class="['account-row', `opens-${horizontal}`]" @mouseleave="menuOpen = false">
    <div class="identity">
      <button
        class="switch-orb"
        :class="{ switching }"
        :disabled="switching"
        :aria-label="`切换到 ${account.alias || account.email}`"
        :title="`切换到 ${account.alias || account.email}`"
        @click.stop="emit('switch', account)"
      >
        <span class="orb-letter">{{ (account.alias || account.email).charAt(0).toUpperCase() }}</span>
        <span class="orb-swap">⇄</span>
      </button>
    </div>
    <div class="account-content">
      <header>
        <span class="account-title">
          <strong>{{ account.alias || account.email }}</strong>
          <span class="plan">{{ planLabel(account.planType) }}</span>
        </span>
        <button class="more" aria-label="账号操作" title="账号操作" @click.stop="menuOpen = !menuOpen">•••</button>
      </header>
      <QuotaRows :five-hour="account.fiveHour" :weekly="account.weekly" />
    </div>
    <div v-if="menuOpen" class="row-menu">
      <button @click.stop="menuOpen = false; emit('remove', account)">删除账号</button>
    </div>
  </article>
</template>

<style scoped>
.account-row {
  position: relative; width: 416px; height: 80px; min-height: 80px; display: grid;
  grid-template-columns: 80px 300px; align-items: center; padding-right: 36px;
}
.account-row::before { content: ""; position: absolute; z-index: 0; top: 0; left: 10px; right: 10px; height: 1px; background: rgba(145, 161, 183, .25); }
.account-row.opens-left { grid-template-columns: 300px 80px; padding: 0 0 0 36px; }
.account-row.opens-left .identity { grid-column: 2; grid-row: 1; }
.account-row.opens-left .account-content { grid-column: 1; grid-row: 1; }
.identity { width: 80px; height: 80px; display: grid; place-items: center; }
.switch-orb {
  position: relative; width: 72px; height: 72px; padding: 0; border: 1px solid rgba(154, 174, 202, .28); border-radius: 50%;
  color: #eef5fc; background: radial-gradient(circle at 36% 28%, #2d394c 0, #182231 56%, #0e1620 100%);
  box-shadow: inset 0 0 0 4px #222d3c, inset 0 0 0 6px rgba(92, 111, 139, .55); cursor: pointer;
  transition: border-color .2s ease, box-shadow .24s ease, transform .2s ease;
}
.switch-orb::after { content: ""; position: absolute; inset: 7px; border: 2px solid rgba(77, 215, 255, .24); border-radius: 50%; transition: border-color .2s ease, box-shadow .2s ease; }
.switch-orb:hover { border-color: rgba(77, 215, 255, .7); box-shadow: inset 0 0 0 4px #202d3c, inset 0 0 0 6px rgba(77, 215, 255, .72), 0 0 10px rgba(77, 215, 255, .22); transform: scale(1.025); }
.switch-orb:hover::after { border-color: rgba(154, 123, 255, .74); box-shadow: 0 0 7px rgba(154, 123, 255, .25); }
.orb-letter, .orb-swap { position: absolute; inset: 0; z-index: 1; display: grid; place-items: center; font-weight: 680; transition: opacity .16s ease, transform .2s ease; }
.orb-letter { font-size: 22px; }
.orb-swap { opacity: 0; color: #67dcff; font-size: 24px; transform: rotate(-20deg) scale(.72); }
.switch-orb:hover .orb-letter, .switch-orb.switching .orb-letter { opacity: 0; transform: scale(.72); }
.switch-orb:hover .orb-swap, .switch-orb.switching .orb-swap { opacity: 1; transform: none; }
.switch-orb.switching .orb-swap { animation: switch-pulse .8s ease-in-out infinite; }
.account-content { width: 300px; display: grid; gap: 7px; min-width: 0; }
header { height: 18px; display: grid; grid-template-columns: minmax(0, 1fr) 28px; align-items: center; column-gap: 6px; min-width: 0; }
.account-title { min-width: 0; display: flex; align-items: center; gap: 6px; }
strong { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
.plan { flex: 0 0 auto; border: 1px solid #8062e8; color: #ece7ff; background: #30225e; border-radius: 6px; padding: 2px 7px; font-size: 10px; font-weight: 750; }
.more { justify-self: end; width: 28px; height: 22px; border: 0; border-radius: 6px; color: #8796aa; background: transparent; cursor: pointer; letter-spacing: -1px; opacity: .52; transition: opacity .15s ease, background .15s ease; }
.account-row:hover .more, .more:focus-visible { opacity: 1; }
.more:hover { color: #e7eef8; background: rgba(255, 255, 255, .07); }
.row-menu { position: absolute; z-index: 5; top: 28px; right: 10px; padding: 4px; border: 1px solid #46556b; border-radius: 9px; background: #202b3a; box-shadow: 0 8px 20px rgba(0, 0, 0, .32); }
.opens-left .row-menu { right: 80px; }
.row-menu button { height: 28px; padding: 0 11px; border: 0; border-radius: 6px; color: #ffb6ae; background: transparent; font-size: 11px; cursor: pointer; }
.row-menu button:hover { background: rgba(255, 98, 109, .12); }
@keyframes switch-pulse { 50% { opacity: .45; transform: scale(.84); } }
</style>
