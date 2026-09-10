<script setup lang="ts">
import type { UserSettings } from "../types/account";

const props = defineProps<{
  settings: UserSettings;
  saving: boolean;
  refreshing: boolean;
  refreshStatus?: string;
}>();
const emit = defineEmits<{
  update: [settings: UserSettings];
  refresh: [];
}>();

function update(patch: Partial<UserSettings>) {
  emit("update", { ...props.settings, ...patch });
}
</script>

<template>
  <section class="settings-view">
    <div class="setting-row">
      <span><strong>自动刷新</strong><small>{{ refreshStatus || "后台串行更新所有账号用量" }}</small></span>
      <div class="refresh-controls">
        <select
          :value="settings.refreshIntervalMinutes"
          :disabled="saving"
          @change="update({ refreshIntervalMinutes: Number(($event.target as HTMLSelectElement).value) })"
        >
          <option :value="1">每 1 分钟</option>
          <option :value="5">每 5 分钟</option>
          <option :value="10">每 10 分钟</option>
          <option :value="15">每 15 分钟</option>
          <option :value="30">每 30 分钟</option>
        </select>
        <button class="refresh-now" :class="{ refreshing }" :disabled="saving || refreshing" title="立即刷新全部账号" aria-label="立即刷新全部账号" @click="emit('refresh')">
          <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M15.4 7.1A6 6 0 1 0 16 11M15.4 7.1V3.8M15.4 7.1h-3.3" /></svg>
        </button>
      </div>
    </div>
    <label class="setting-row">
      <span><strong>开机启动</strong><small>登录 Windows 后自动显示悬浮球</small></span>
      <span class="toggle">
        <input
          type="checkbox"
          :checked="settings.launchAtLogin"
          :disabled="saving"
          aria-label="开机启动"
          @change="update({ launchAtLogin: ($event.target as HTMLInputElement).checked })"
        />
        <i />
      </span>
    </label>
    <label class="setting-row">
      <span><strong>记住悬浮球位置</strong><small>按显示器与缩放比例分别保存</small></span>
      <span class="toggle">
        <input
          type="checkbox"
          :checked="settings.rememberPosition"
          :disabled="saving"
          aria-label="记住悬浮球位置"
          @change="update({ rememberPosition: ($event.target as HTMLInputElement).checked })"
        />
        <i />
      </span>
    </label>
  </section>
</template>

<style scoped>
.settings-view { flex: 1; min-height: 0; overflow: hidden; padding: 0 20px; border-top: 1px solid rgba(145, 161, 183, .24); }
.setting-row { min-height: 62px; display: flex; align-items: center; justify-content: space-between; gap: 14px; border-bottom: 1px solid rgba(145, 161, 183, .15); }
.setting-row > span { min-width: 0; display: grid; gap: 3px; }
strong { color: #eef4fc; font-size: 13px; font-weight: 650; }
small { color: #8997aa; font-size: 11px; }
.refresh-controls { flex: 0 0 auto; display: flex; align-items: center; gap: 7px; }
select { width: 108px; color: #e6edf7; background: #1d2837; border: 1px solid #43536a; border-radius: 8px; padding: 7px 9px; outline: none; }
.toggle { position: relative; flex: 0 0 34px; width: 34px; height: 19px; }
.toggle input { position: absolute; inset: 0; z-index: 2; margin: 0; opacity: 0; cursor: pointer; }
.toggle i { position: absolute; inset: 0; border: 1px solid #536279; border-radius: 999px; background: #263244; transition: background .16s ease, border-color .16s ease; }
.toggle i::after { content: ""; position: absolute; top: 2px; left: 2px; width: 13px; height: 13px; border-radius: 50%; background: #aeb9c8; transition: transform .16s ease, background .16s ease; }
.toggle input:checked + i { border-color: #269dff; background: #087ef5; }
.toggle input:checked + i::after { transform: translateX(15px); background: white; }
.toggle input:focus-visible + i { outline: 2px solid rgba(77, 215, 255, .7); outline-offset: 2px; }
.refresh-now { width: 32px; height: 32px; padding: 6px; border: 1px solid #43536a; border-radius: 8px; color: #72dcff; background: #1d2837; cursor: pointer; }
.refresh-now svg { width: 100%; height: 100%; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
.refresh-now:hover { border-color: #288fd0; background: #223348; }
.refresh-now.refreshing svg { animation: spin .8s linear infinite; }
.refresh-now:disabled, select:disabled, .toggle:has(input:disabled) { opacity: .55; cursor: default; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
