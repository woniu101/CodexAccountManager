<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
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
const pendingSwitch = ref<ManagedAccount>();
const settings = ref<UserSettings>({
  refreshIntervalMinutes: 5,
  launchAtLogin: false,
  rememberPosition: true,
});
let hoverTimer: number | undefined;
let refreshTimer: number | undefined;
const unlisteners: UnlistenFn[] = [];
let dragStart: {
  pointerId: number;
  screenX: number;
  screenY: number;
  windowX: number;
  windowY: number;
  target: HTMLElement;
  moved: boolean;
} | undefined;
let pendingDragPosition: PhysicalPosition | undefined;
let dragFrame: number | undefined;
let dragWriteInFlight = false;

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
  if (pendingSwitch.value) return 224;
  if (settingsOpen.value) return 304;
  if (!store.accounts.length) return 230;
  return 52 + 94 * Math.min(store.accounts.length, 3);
});

async function setMode(nextMode: WindowMode, force = false) {
  window.clearTimeout(hoverTimer);
  if (mode.value === nextMode && !force) return;
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
  if (mode.value !== "idle" || dragStart) return;
  hoverTimer = window.setTimeout(() => void setMode("hover"), 220);
}
function scheduleIdle() {
  if (mode.value !== "hover" || dragStart) return;
  hoverTimer = window.setTimeout(() => void setMode("idle"), 260);
}
function cancelHoverTimer() { window.clearTimeout(hoverTimer); }

async function beginRightDrag(event: PointerEvent) {
  if (event.button !== 2 || dragStart) return;
  event.preventDefault();
  event.stopPropagation();
  cancelHoverTimer();
  try {
    const target = event.currentTarget as HTMLElement;
    const position = await getCurrentWindow().outerPosition();
    target.setPointerCapture(event.pointerId);
    dragStart = {
      pointerId: event.pointerId,
      screenX: event.screenX,
      screenY: event.screenY,
      windowX: position.x,
      windowY: position.y,
      target,
      moved: false,
    };
  } catch (error) {
    store.error = `准备拖动窗口失败：${String(error)}`;
  }
}

function queueDragPosition(event: PointerEvent) {
  if (!dragStart || event.pointerId !== dragStart.pointerId) return;
  event.preventDefault();
  const scale = window.devicePixelRatio || 1;
  const deltaX = (event.screenX - dragStart.screenX) * scale;
  const deltaY = (event.screenY - dragStart.screenY) * scale;
  if (Math.hypot(deltaX, deltaY) > 2) dragStart.moved = true;
  pendingDragPosition = new PhysicalPosition(
    Math.round(dragStart.windowX + deltaX),
    Math.round(dragStart.windowY + deltaY),
  );
  if (dragFrame === undefined) dragFrame = window.requestAnimationFrame(() => void flushDragPosition());
}

async function flushDragPosition() {
  dragFrame = undefined;
  if (dragWriteInFlight || !pendingDragPosition) return;
  const position = pendingDragPosition;
  pendingDragPosition = undefined;
  dragWriteInFlight = true;
  try {
    await getCurrentWindow().setPosition(position);
  } catch (error) {
    store.error = `拖动窗口失败：${String(error)}`;
  } finally {
    dragWriteInFlight = false;
    if (pendingDragPosition && dragFrame === undefined) {
      dragFrame = window.requestAnimationFrame(() => void flushDragPosition());
    }
  }
}

async function endRightDrag(event: PointerEvent) {
  if (!dragStart || event.pointerId !== dragStart.pointerId) return;
  event.preventDefault();
  const { target, pointerId, moved } = dragStart;
  dragStart = undefined;
  if (target.hasPointerCapture(pointerId)) target.releasePointerCapture(pointerId);
  if (!moved) return;
  if (dragFrame !== undefined) {
    window.cancelAnimationFrame(dragFrame);
    dragFrame = undefined;
  }
  await flushDragPosition();
  while (dragWriteInFlight || pendingDragPosition) {
    await new Promise((resolve) => window.setTimeout(resolve, 16));
    await flushDragPosition();
  }
  try {
    await invoke("save_window_position", {
      mode: mode.value,
      horizontal: placement.value.horizontal,
    });
    toast.value = "悬浮窗位置已保存";
    window.setTimeout(() => { toast.value = undefined; }, 1200);
  } catch (error) {
    store.error = `保存窗口位置失败：${String(error)}`;
  }
}

async function addAccount() {
  settingsOpen.value = false;
  await nextTick();
  await setMode("expanded", mode.value === "expanded");
  await store.startAddAccount();
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
    if (settings.value.rememberPosition) {
      await invoke("save_window_position", {
        mode: mode.value,
        horizontal: placement.value.horizontal,
      });
    }
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
  pendingSwitch.value = account;
  await nextTick();
  await setMode("expanded", true);
}
async function closeSwitchConfirm() {
  pendingSwitch.value = undefined;
  await nextTick();
  await setMode("expanded", true);
}
async function confirmSwitch() {
  const account = pendingSwitch.value;
  if (!account) return;
  pendingSwitch.value = undefined;
  await nextTick();
  await setMode("expanded", true);
  await store.switchTo(account);
}
function onKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  if (pendingSwitch.value) void closeSwitchConfirm();
  else void setMode("idle");
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
  <main
    :class="['stage', ...stageClass]"
    @pointerdown="beginRightDrag"
    @pointermove="queueDragPosition"
    @pointerup="endRightDrag"
    @pointercancel="endRightDrag"
    @contextmenu.prevent
  >
    <button
      v-if="mode === 'idle'"
      class="orb-button"
      aria-label="打开 Codex Account Manager"
      @mouseenter="scheduleHover"
      @mouseleave="scheduleIdle"
      @click="setMode('expanded')"
    >
      <QuotaRing :five-hour="store.current?.fiveHour" :weekly="store.current?.weekly" :process-state="processState" />
    </button>

    <div
      v-else-if="mode === 'hover'"
      class="glass hover-shell"
      @mouseenter="cancelHoverTimer"
      @mouseleave="scheduleIdle"
    >
      <HoverSummary :account="store.current" :process-state="processState" :loading="store.loading" @expand="setMode('expanded')" />
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
        :login="store.login"
        :adding-account="store.addingAccount"
        @add="addAccount"
        @settings="showSettings"
        @import-current="store.importCurrent()"
        @switch="switchAccount"
        @update-settings="updateSettings"
        @refresh="store.load()"
        @collapse="setMode('idle')"
        @cancel-login="store.cancelLogin()"
      />
      <div v-if="pendingSwitch" class="confirm-layer" @click.self="closeSwitchConfirm">
        <section class="confirm-card" role="dialog" aria-modal="true" aria-labelledby="switch-title">
          <span class="confirm-icon">⇄</span>
          <div class="confirm-copy">
            <strong id="switch-title">切换 Codex 账号</strong>
            <span>{{ pendingSwitch.alias || pendingSwitch.email }}</span>
            <small>Codex Desktop 将关闭并重新启动，正在运行的任务可能中断。</small>
          </div>
          <div class="confirm-actions">
            <button class="secondary" @click="closeSwitchConfirm">取消</button>
            <button class="primary" @click="confirmSwitch">确认切换</button>
          </div>
        </section>
      </div>
      <p v-if="store.error" class="notice error" :title="store.error">{{ store.error }}</p>
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
