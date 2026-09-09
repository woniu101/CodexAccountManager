<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import AccountPanel from "./components/AccountPanel.vue";
import HoverSummary from "./components/HoverSummary.vue";
import QuotaRing from "./components/QuotaRing.vue";
import { useAccountStore } from "./stores/accounts";
import type { ManagedAccount, WindowPlacement } from "./types/account";

type WindowMode = "idle" | "hover" | "expanded";

const store = useAccountStore();
const mode = ref<WindowMode>("idle");
const placement = ref<WindowPlacement>({ horizontal: "right", vertical: "up" });
const toast = ref<string>();
let hoverTimer: number | undefined;
let refreshTimer: number | undefined;
let unlistenRefresh: UnlistenFn | undefined;
let pointerOrigin: { x: number; y: number } | undefined;
let dragged = false;

const stageClass = computed(() => [
  `mode-${mode.value}`,
  `opens-${placement.value.horizontal}`,
  `opens-${placement.value.vertical}`,
]);

async function setMode(nextMode: WindowMode) {
  window.clearTimeout(hoverTimer);
  if (mode.value === nextMode) return;
  if (nextMode === "idle") {
    mode.value = nextMode;
    await nextTick();
  }
  try {
    placement.value = await invoke<WindowPlacement>("set_window_mode", {
      mode: nextMode,
      currentHorizontal: placement.value.horizontal,
      currentVertical: placement.value.vertical,
    });
  } catch (error) {
    store.error = String(error);
  }
  mode.value = nextMode;
}

function scheduleHover() {
  if (mode.value !== "idle" || pointerOrigin) return;
  hoverTimer = window.setTimeout(() => void setMode("hover"), 150);
}
function scheduleIdle() {
  if (mode.value !== "hover") return;
  hoverTimer = window.setTimeout(() => void setMode("idle"), 300);
}
function cancelHoverTimer() { window.clearTimeout(hoverTimer); }

function beginPointer(event: PointerEvent) {
  if (mode.value !== "idle" || event.button !== 0) return;
  pointerOrigin = { x: event.screenX, y: event.screenY };
  dragged = false;
  cancelHoverTimer();
  window.addEventListener("pointermove", trackPointer);
  window.addEventListener("pointerup", endPointer, { once: true });
}
async function trackPointer(event: PointerEvent) {
  if (!pointerOrigin || dragged) return;
  if (Math.hypot(event.screenX - pointerOrigin.x, event.screenY - pointerOrigin.y) > 4) {
    dragged = true;
    await getCurrentWindow().startDragging();
  }
}
function endPointer() {
  window.removeEventListener("pointermove", trackPointer);
  if (!dragged) void setMode("expanded");
  pointerOrigin = undefined;
  window.setTimeout(() => { dragged = false; }, 0);
}

async function addAccount() {
  await store.startAddAccount();
  if (store.login.status === "waiting") toast.value = "已打开浏览器，正在等待授权…";
}
function showSettings() {
  toast.value = "当前每 5 分钟自动刷新；更多设置将在下一阶段开放";
  window.setTimeout(() => { toast.value = undefined; }, 3200);
}
async function switchAccount(account: ManagedAccount) {
  const confirmed = window.confirm(
    `切换到 ${account.alias || account.email}？\n\n这会关闭并重新启动 Codex Desktop，可能中断正在运行的任务。`,
  );
  if (confirmed) await store.switchTo(account);
}
function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") void setMode("idle");
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  await store.load();
  refreshTimer = window.setInterval(() => void store.load(), 5 * 60 * 1000);
  unlistenRefresh = await listen("refresh-requested", () => void store.load());
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.clearInterval(refreshTimer);
  cancelHoverTimer();
  unlistenRefresh?.();
});
</script>

<template>
  <main :class="['stage', ...stageClass]">
    <button
      v-if="mode === 'idle'"
      class="orb-button"
      aria-label="打开 Codex Account Manager"
      @mouseenter="scheduleHover"
      @mouseleave="scheduleIdle"
      @pointerdown="beginPointer"
    >
      <QuotaRing :five-hour="store.current?.fiveHour" :weekly="store.current?.weekly" :codex-running="store.codexRunning" />
    </button>

    <div v-else-if="mode === 'hover'" class="glass hover-shell" @mouseenter="cancelHoverTimer" @mouseleave="scheduleIdle">
      <HoverSummary :account="store.current" :codex-running="store.codexRunning" :loading="store.loading" @expand="setMode('expanded')" />
    </div>

    <div v-else class="glass expanded-shell">
      <AccountPanel
        :accounts="store.accounts"
        :codex-running="store.codexRunning"
        :switching-id="store.switchingId"
        :loading="store.loading"
        @add="addAccount"
        @settings="showSettings"
        @import-current="store.importCurrent()"
        @switch="switchAccount"
      />
      <button class="collapse" title="收起" @click="setMode('idle')">⌄</button>
      <p v-if="store.error" class="notice error" :title="store.error">{{ store.error }}</p>
      <p v-else-if="store.login.status === 'waiting'" class="notice">
        等待浏览器授权… <button @click="store.cancelLogin()">取消</button>
      </p>
      <p v-else-if="store.login.status === 'failed'" class="notice error">{{ store.login.message }}</p>
      <p v-else-if="toast" class="notice">{{ toast }}</p>
    </div>
  </main>
</template>
