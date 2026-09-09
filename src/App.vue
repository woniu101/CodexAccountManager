<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import AccountPanel from "./components/AccountPanel.vue";
import HoverSummary from "./components/HoverSummary.vue";
import QuotaRing from "./components/QuotaRing.vue";
import { useAccountStore } from "./stores/accounts";
import type { ManagedAccount, ProcessState, UserSettings, WindowPlacement } from "./types/account";

type WindowMode = "idle" | "hover" | "expanded";

const store = useAccountStore();
const mode = ref<WindowMode>("idle");
const placement = ref<WindowPlacement>({ horizontal: "right", vertical: "up" });
const toast = ref<string>();
const switchStage = ref<string>();
const settingsOpen = ref(false);
const savingSettings = ref(false);
const settings = ref<UserSettings>({
  refreshIntervalMinutes: 5,
  launchAtLogin: false,
  rememberPosition: true,
});
let hoverTimer: number | undefined;
let refreshTimer: number | undefined;
const unlisteners: UnlistenFn[] = [];
let pointerOrigin: { x: number; y: number } | undefined;
let pointerMode: WindowMode = "idle";
let dragged = false;
let suppressHoverUntil = 0;
let suppressClickUntil = 0;

const stageClass = computed(() => [
  `mode-${mode.value}`,
  `opens-${placement.value.horizontal}`,
  `opens-${placement.value.vertical}`,
]);
const processState = computed<ProcessState>(() => {
  if (store.switchingId) return "switching";
  if (store.error) return "error";
  return store.codexRunning ? "running" : "stopped";
});
const expandedSurfaceHeight = computed(() => {
  if (settingsOpen.value) return 320;
  if (!store.accounts.length) return 260;
  return 56 + 102 * Math.min(store.accounts.length, 3);
});

async function setMode(nextMode: WindowMode, force = false) {
  window.clearTimeout(hoverTimer);
  if (mode.value === nextMode && !force) return;
  if (nextMode === "expanded" && mode.value === "idle") {
    await setMode("hover");
    await new Promise((resolve) => window.setTimeout(resolve, 70));
  }
  if (nextMode === "idle") {
    mode.value = nextMode;
    await nextTick();
  }
  try {
    placement.value = await invoke<WindowPlacement>("set_window_mode", {
      mode: nextMode,
      currentHorizontal: placement.value.horizontal,
      currentVertical: placement.value.vertical,
      expandedHeight: expandedSurfaceHeight.value + 8,
    });
  } catch (error) {
    store.error = String(error);
  }
  mode.value = nextMode;
  if (nextMode === "expanded") {
    const cacheAge = Date.now() / 1000 - (store.refreshedAt ?? 0);
    if (cacheAge > 60 && !store.loading) void store.load();
  }
}

function scheduleHover() {
  if (mode.value !== "idle" || pointerOrigin || Date.now() < suppressHoverUntil) return;
  hoverTimer = window.setTimeout(() => void setMode("hover"), 260);
}
function scheduleIdle() {
  if (mode.value !== "hover") return;
  hoverTimer = window.setTimeout(() => void setMode("idle"), 300);
}
function cancelHoverTimer() { window.clearTimeout(hoverTimer); }

function beginPointer(event: PointerEvent) {
  if (event.button !== 0 || mode.value === "expanded") return;
  if (
    mode.value === "hover"
    && !event.composedPath().some((item) => item instanceof HTMLElement && item.classList.contains("quota-orb"))
  ) return;
  event.preventDefault();
  pointerMode = mode.value;
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
    pointerOrigin = undefined;
    window.removeEventListener("pointermove", trackPointer);
    window.removeEventListener("pointerup", endPointer);
    try {
      await getCurrentWindow().startDragging();
      if (pointerMode === "hover") await setMode("idle");
      await invoke("save_window_position");
    } catch (error) {
      store.error = `拖动悬浮球失败：${String(error)}`;
    } finally {
      suppressHoverUntil = Date.now() + 500;
      suppressClickUntil = Date.now() + 500;
      window.setTimeout(() => { dragged = false; }, 0);
    }
  }
}

function expandFromSurface() {
  if (Date.now() >= suppressClickUntil) void setMode("expanded");
}
function endPointer() {
  window.removeEventListener("pointermove", trackPointer);
  window.removeEventListener("pointerup", endPointer);
  const shouldExpand = Boolean(pointerOrigin) && !dragged;
  pointerOrigin = undefined;
  window.setTimeout(() => { dragged = false; }, 0);
  if (shouldExpand) void setMode("expanded");
}

async function addAccount() {
  settingsOpen.value = false;
  await nextTick();
  await setMode("expanded", mode.value === "expanded");
  await store.startAddAccount();
  if (store.login.status === "waiting") toast.value = "已打开浏览器，正在等待授权…";
}
async function showSettings() {
  settingsOpen.value = !settingsOpen.value;
  await nextTick();
  await setMode("expanded", mode.value === "expanded");
}
function scheduleRefresh() {
  window.clearInterval(refreshTimer);
  refreshTimer = window.setInterval(
    () => void store.load(),
    settings.value.refreshIntervalMinutes * 60 * 1000,
  );
}
async function updateSettings(next: UserSettings) {
  savingSettings.value = true;
  try {
    settings.value = await invoke<UserSettings>("update_settings", { settings: next });
    scheduleRefresh();
    toast.value = "设置已保存";
    window.setTimeout(() => { toast.value = undefined; }, 1800);
  } catch (error) {
    store.error = String(error);
  } finally {
    savingSettings.value = false;
  }
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
  try {
    settings.value = await invoke<UserSettings>("get_settings");
  } catch (error) {
    store.error = String(error);
  }
  await store.load();
  scheduleRefresh();
  unlisteners.push(await listen("refresh-requested", () => void store.load()));
  unlisteners.push(await listen("add-account-requested", () => void addAccount()));
  unlisteners.push(await listen("settings-requested", () => {
    void (async () => {
      settingsOpen.value = true;
      await nextTick();
      await setMode("expanded", mode.value === "expanded");
    })();
  }));
  unlisteners.push(await listen<string>("switch-progress", (event) => {
    switchStage.value = event.payload;
  }));
});
watch(() => store.accounts.length, () => {
  if (mode.value === "expanded" && !settingsOpen.value) {
    void setMode("expanded", true);
  }
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.clearInterval(refreshTimer);
  cancelHoverTimer();
  for (const unlisten of unlisteners) unlisten();
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
      <QuotaRing :five-hour="store.current?.fiveHour" :weekly="store.current?.weekly" :process-state="processState" />
    </button>

    <div
      v-else-if="mode === 'hover'"
      class="glass hover-shell"
      @mouseenter="cancelHoverTimer"
      @mouseleave="scheduleIdle"
      @pointerdown="beginPointer"
    >
      <HoverSummary :account="store.current" :process-state="processState" :loading="store.loading" @expand="expandFromSurface" />
    </div>

    <div v-else class="glass expanded-shell" :style="{ height: `${expandedSurfaceHeight}px` }">
      <AccountPanel
        :accounts="store.accounts"
        :process-state="processState"
        :switching-id="store.switchingId"
        :loading="store.loading"
        :settings-open="settingsOpen"
        :settings="settings"
        :saving-settings="savingSettings"
        @add="addAccount"
        @settings="showSettings"
        @import-current="store.importCurrent()"
        @switch="switchAccount"
        @update-settings="updateSettings"
        @refresh="store.load()"
        @collapse="setMode('idle')"
      />
      <p v-if="store.error" class="notice error" :title="store.error">{{ store.error }}</p>
      <p v-else-if="store.login.status === 'waiting'" class="notice">
        等待浏览器授权… <button @click="store.cancelLogin()">取消</button>
      </p>
      <p v-else-if="store.login.status === 'duplicate'" class="notice">
        账号 {{ store.login.account.email }} 已存在，是否更新凭据？
        <button @click="store.confirmDuplicate(true)">更新</button>
        <button @click="store.confirmDuplicate(false)">取消</button>
      </p>
      <p v-else-if="store.login.status === 'failed'" class="notice error">{{ store.login.message }}</p>
      <p v-else-if="store.switchingId && switchStage" class="notice switching">{{ switchStage }}</p>
      <p v-else-if="toast" class="notice">{{ toast }}</p>
    </div>
  </main>
</template>
