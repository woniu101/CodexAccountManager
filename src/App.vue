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
const surfaceAppearing = ref(false);
const surfaceCollapsing = ref(false);
const surfaceStaged = ref(false);
const panelRevealing = ref(false);
const switchStage = ref<string>();
const settingsOpen = ref(false);
const savingSettings = ref(false);
const settingsSaved = ref(false);
const orbSaved = ref(false);
const refreshStatus = ref<string>();
const pendingSwitch = ref<ManagedAccount>();
const pendingDelete = ref<ManagedAccount>();
const settings = ref<UserSettings>({
  refreshIntervalMinutes: 5,
  launchAtLogin: false,
  rememberPosition: true,
});
let hoverTimer: number | undefined;
let surfaceTimer: number | undefined;
let feedbackTimer: number | undefined;
let modeTransition = 0;
let appliedExpandedHeight = 0;
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
const statusMessage = computed(() => (
  store.error
  || (store.switchingId ? switchStage.value : undefined)
  || store.notice
  || (!settingsOpen.value ? refreshStatus.value : undefined)
));
const statusTone = computed<"error" | "switching" | "info">(() => store.error ? "error" : store.switchingId ? "switching" : "info");
const hasPanelNotice = computed(() => Boolean(statusMessage.value || store.login.status === "duplicate" || store.login.status === "failed"));
const expandedSurfaceHeight = computed(() => {
  if (pendingSwitch.value || pendingDelete.value) return 224;
  if (settingsOpen.value) return 318 + (hasPanelNotice.value ? 32 : 0);
  if (!store.accounts.length) return 230;
  const otherRows = Math.min(Math.max(store.accounts.length - 1, 0), 2);
  return 52 + 80 + 80 * otherRows + (hasPanelNotice.value ? 32 : 0);
});

async function setMode(nextMode: WindowMode, force = false, refreshPlacement = false) {
  const targetExpandedHeight = expandedSurfaceHeight.value + 8;
  if (mode.value === nextMode) {
    if (!force) return;
    if (
      nextMode === "expanded"
      && !refreshPlacement
      && appliedExpandedHeight === targetExpandedHeight
    ) return;
  }
  const transition = ++modeTransition;
  window.clearTimeout(hoverTimer);
  window.clearTimeout(surfaceTimer);
  surfaceAppearing.value = false;
  const previousMode = mode.value;
  const expanding = (
    (previousMode === "idle" && nextMode !== "idle")
    || (previousMode === "hover" && nextMode === "expanded")
  );
  panelRevealing.value = false;
  if (nextMode === "idle" && previousMode !== "idle") {
    surfaceCollapsing.value = true;
    await new Promise((resolve) => window.setTimeout(resolve, 280));
    if (transition !== modeTransition) {
      surfaceCollapsing.value = false;
      return;
    }
  }
  if (expanding) {
    surfaceStaged.value = true;
    mode.value = nextMode;
    await nextTick();
  }
  try {
    placement.value = await invoke<WindowPlacement>("set_window_mode", {
      mode: nextMode,
      currentMode: previousMode,
      currentHorizontal: placement.value.horizontal,
      currentVertical: placement.value.vertical,
      expandedHeight: targetExpandedHeight,
    });
  } catch (error) {
    if (expanding) mode.value = previousMode;
    surfaceStaged.value = false;
    store.error = String(error);
    surfaceCollapsing.value = false;
    return;
  }
  if (!expanding) mode.value = nextMode;
  if (nextMode === "expanded") appliedExpandedHeight = targetExpandedHeight;
  surfaceCollapsing.value = false;
  if (expanding) {
    await nextTick();
    surfaceStaged.value = false;
  }
  if (previousMode === "idle" && nextMode !== "idle") {
    surfaceAppearing.value = true;
    surfaceTimer = window.setTimeout(() => { surfaceAppearing.value = false; }, 240);
  } else if (previousMode === "hover" && nextMode === "expanded") {
    panelRevealing.value = true;
    surfaceTimer = window.setTimeout(() => { panelRevealing.value = false; }, 240);
  }
  if (nextMode === "expanded") {
    const cacheAge = Date.now() / 1000 - (store.refreshedAt ?? 0);
    if (cacheAge > 60 && !store.loading) void store.load();
  }
}

function scheduleHover() {
  if (mode.value !== "idle" || dragStart) return;
  hoverTimer = window.setTimeout(() => void setMode("hover"), 180);
}
function scheduleIdle() {
  if (mode.value === "idle" || dragStart || pendingSwitch.value || pendingDelete.value) return;
  const delay = mode.value === "hover" ? 280 : 800;
  hoverTimer = window.setTimeout(() => {
    const focused = document.activeElement;
    if (
      mode.value === "expanded"
      && focused instanceof HTMLElement
      && focused.matches("input, select, textarea")
    ) return;
    void setMode("idle");
  }, delay);
}
function cancelHoverTimer() {
  window.clearTimeout(hoverTimer);
  if (surfaceCollapsing.value) {
    modeTransition += 1;
    surfaceCollapsing.value = false;
  }
}

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
    await setMode(mode.value, true, true);
    await invoke("save_window_position", {
      mode: mode.value,
      horizontal: placement.value.horizontal,
      vertical: placement.value.vertical,
    });
    orbSaved.value = true;
    window.clearTimeout(feedbackTimer);
    feedbackTimer = window.setTimeout(() => { orbSaved.value = false; }, 950);
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
  window.clearTimeout(refreshTimer);
  refreshTimer = window.setTimeout(() => {
    void (async () => {
      await store.load();
      scheduleRefresh();
    })();
  }, settings.value.refreshIntervalMinutes * 60 * 1000);
}
async function updateSettings(next: UserSettings) {
  savingSettings.value = true;
  try {
    settings.value = await invoke<UserSettings>("update_settings", { settings: next });
    if (settings.value.rememberPosition) {
      await invoke("save_window_position", {
        mode: mode.value,
        horizontal: placement.value.horizontal,
        vertical: placement.value.vertical,
      });
    }
    scheduleRefresh();
    settingsSaved.value = true;
    window.clearTimeout(feedbackTimer);
    feedbackTimer = window.setTimeout(() => { settingsSaved.value = false; }, 1200);
  } catch (error) {
    store.error = String(error);
  } finally {
    savingSettings.value = false;
  }
}
async function refreshAccounts() {
  refreshStatus.value = "正在刷新全部账号…";
  await store.load(true);
  refreshStatus.value = store.error ? "刷新失败，请查看提示" : "刚刚完成刷新";
  scheduleRefresh();
  window.clearTimeout(feedbackTimer);
  feedbackTimer = window.setTimeout(() => { refreshStatus.value = undefined; }, 1800);
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
async function requestDelete(account: ManagedAccount) {
  pendingDelete.value = account;
  await nextTick();
  await setMode("expanded", true);
}
async function closeDeleteConfirm() {
  pendingDelete.value = undefined;
  await nextTick();
  await setMode("expanded", true);
}
async function confirmDelete() {
  const account = pendingDelete.value;
  if (!account) return;
  pendingDelete.value = undefined;
  await store.removeAccount(account);
  await nextTick();
  await setMode("expanded", true);
}
function onKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  if (pendingSwitch.value) void closeSwitchConfirm();
  else if (pendingDelete.value) void closeDeleteConfirm();
  else void setMode("idle");
}

function handleOrbClick() {
  if (mode.value === "expanded") void setMode("idle");
  else void setMode("expanded");
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  try {
    settings.value = await invoke<UserSettings>("get_settings");
  } catch (error) {
    store.error = String(error);
  }
  await setMode("idle", true, true);
  await getCurrentWindow().show();
  await store.load();
  scheduleRefresh();
  unlisteners.push(await listen("refresh-requested", () => void refreshAccounts()));
  unlisteners.push(await listen("add-account-requested", () => void addAccount()));
  unlisteners.push(await listen("settings-requested", () => {
    void (async () => {
      settingsOpen.value = true;
      await nextTick();
      await setMode("expanded", mode.value === "expanded");
    })();
  }));
  unlisteners.push(await listen<string>("switch-account-requested", (event) => {
    void (async () => {
      let account = store.accounts.find((item) => item.id === event.payload);
      if (!account) {
        await store.load();
        account = store.accounts.find((item) => item.id === event.payload);
      }
      if (account && !account.isActive) await switchAccount(account);
    })();
  }));
  unlisteners.push(await listen<string>("switch-progress", (event) => {
    switchStage.value = event.payload;
  }));
});
watch([() => store.accounts.length, hasPanelNotice], () => {
  if (mode.value === "expanded" && !settingsOpen.value) {
    void setMode("expanded", true);
  }
});
watch(() => store.notice, (notice) => {
  if (notice) void setMode("expanded", true);
});
watch(() => store.login.status, (status) => {
  if (status === "failed") void setMode("expanded", true);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.clearTimeout(refreshTimer);
  window.clearTimeout(surfaceTimer);
  window.clearTimeout(feedbackTimer);
  cancelHoverTimer();
  for (const unlisten of unlisteners) unlisten();
});
</script>

<template>
  <main
    :class="['stage', ...stageClass, { 'has-modal': pendingSwitch || pendingDelete }]"
    @pointerdown="beginRightDrag"
    @pointermove="queueDragPosition"
    @pointerup="endRightDrag"
    @pointercancel="endRightDrag"
    @contextmenu.prevent
  >
    <div
      :class="[
        'glass',
        'surface',
        mode === 'expanded' ? 'expanded-shell' : 'hover-shell',
        {
          'surface-appearing': surfaceAppearing,
          'surface-collapsing': surfaceCollapsing,
          'surface-staged': surfaceStaged,
          'panel-revealing': panelRevealing,
        },
      ]"
      :style="{ height: `${mode === 'expanded' ? expandedSurfaceHeight : 80}px` }"
      @mouseenter="cancelHoverTimer"
      @mouseleave="scheduleIdle"
      @click="mode === 'hover' && setMode('expanded')"
    >
      <HoverSummary
        class="anchor-summary"
        :account="store.current"
        :loading="store.loading"
        :horizontal="placement.horizontal"
      />
      <AccountPanel
        :accounts="store.accounts"
        :switching-id="store.switchingId"
        :loading="store.loading"
        :settings-open="settingsOpen"
        :settings="settings"
        :saving-settings="savingSettings"
        :settings-saved="settingsSaved"
        :refresh-status="refreshStatus"
        :status-message="statusMessage"
        :status-tone="statusTone"
        :login="store.login"
        :adding-account="store.addingAccount"
        :cancelling-login="store.cancellingLogin"
        :horizontal="placement.horizontal"
        :vertical="placement.vertical"
        @add="addAccount"
        @settings="showSettings"
        @import-current="store.importCurrent()"
        @switch="switchAccount"
        @remove="requestDelete"
        @update-settings="updateSettings"
        @refresh="refreshAccounts"
        @cancel-login="store.cancelLogin()"
        @confirm-duplicate="store.confirmDuplicate($event)"
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
      <div v-if="pendingDelete" class="confirm-layer" @click.self="closeDeleteConfirm">
        <section class="confirm-card danger-card" role="dialog" aria-modal="true" aria-labelledby="delete-title">
          <span class="confirm-icon danger">×</span>
          <div class="confirm-copy">
            <strong id="delete-title">删除本地账号？</strong>
            <span class="account-chip">{{ pendingDelete.alias || pendingDelete.email }}</span>
            <small>将移除本机保存的账号与加密凭据，不影响 OpenAI 账号，之后仍可重新添加。</small>
          </div>
          <div class="confirm-actions">
            <button class="secondary" @click="closeDeleteConfirm">保留账号</button>
            <button class="danger-action" @click="confirmDelete">删除</button>
          </div>
        </section>
      </div>
    </div>

    <button
      class="orb-button persistent-orb"
      :aria-label="mode === 'expanded' ? '收起 Codex Account Manager' : '打开 Codex Account Manager'"
      @mouseenter="mode === 'idle' ? scheduleHover() : cancelHoverTimer()"
      @mouseleave="scheduleIdle"
      @click.stop="handleOrbClick"
    >
      <QuotaRing
        :five-hour="store.current?.fiveHour"
        :weekly="store.current?.weekly"
        :process-state="processState"
        :saved="orbSaved"
      />
    </button>
  </main>
</template>
