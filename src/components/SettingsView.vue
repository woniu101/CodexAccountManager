<script setup lang="ts">
import type { UserSettings } from "../types/account";

const props = defineProps<{ settings: UserSettings; saving: boolean }>();
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
      <span><strong>自动刷新</strong><small>后台串行更新所有账号用量</small></span>
      <select
        :value="settings.refreshIntervalMinutes"
        :disabled="saving"
        @change="update({ refreshIntervalMinutes: Number(($event.target as HTMLSelectElement).value) })"
      >
        <option :value="5">每 5 分钟</option>
        <option :value="10">每 10 分钟</option>
        <option :value="15">每 15 分钟</option>
        <option :value="30">每 30 分钟</option>
      </select>
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
    <button class="refresh-now" :disabled="saving" @click="emit('refresh')">
      立即刷新全部账号
    </button>
    <p>在任意状态按住右键拖动窗口，左键仅用于展开或操作。</p>
  </section>
</template>

<style scoped>
.settings-view { flex: 1; min-height: 0; padding: 10px 22px 18px; border-top: 1px solid rgba(145, 161, 183, .24); }
.setting-row { min-height: 64px; display: flex; align-items: center; justify-content: space-between; gap: 20px; border-bottom: 1px solid rgba(145, 161, 183, .15); }
.setting-row > span { display: grid; gap: 5px; }
strong { color: #eef4fc; font-size: 13px; font-weight: 650; }
small, p { color: #8997aa; font-size: 11px; }
select { min-width: 108px; color: #e6edf7; background: #1d2837; border: 1px solid #43536a; border-radius: 8px; padding: 7px 9px; outline: none; }
.toggle { position: relative; flex: 0 0 34px; width: 34px; height: 19px; }
.toggle input { position: absolute; inset: 0; z-index: 2; margin: 0; opacity: 0; cursor: pointer; }
.toggle i { position: absolute; inset: 0; border: 1px solid #536279; border-radius: 999px; background: #263244; transition: background .16s ease, border-color .16s ease; }
.toggle i::after { content: ""; position: absolute; top: 2px; left: 2px; width: 13px; height: 13px; border-radius: 50%; background: #aeb9c8; transition: transform .16s ease, background .16s ease; }
.toggle input:checked + i { border-color: #269dff; background: #087ef5; }
.toggle input:checked + i::after { transform: translateX(15px); background: white; }
.toggle input:focus-visible + i { outline: 2px solid rgba(77, 215, 255, .7); outline-offset: 2px; }
.refresh-now { display: block; margin: 17px auto 0; padding: 8px 18px; border: 1px solid #1685ff; border-radius: 8px; color: white; background: #087ef5; cursor: pointer; }
.refresh-now:hover { background: #2392ff; }
.refresh-now:disabled, select:disabled, .toggle:has(input:disabled) { opacity: .55; cursor: default; }
p { margin: 12px 0 0; text-align: center; }
</style>
