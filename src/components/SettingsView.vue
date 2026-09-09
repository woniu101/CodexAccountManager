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
      <input
        type="checkbox"
        :checked="settings.launchAtLogin"
        :disabled="saving"
        @change="update({ launchAtLogin: ($event.target as HTMLInputElement).checked })"
      />
    </label>
    <label class="setting-row">
      <span><strong>记住悬浮球位置</strong><small>按显示器与缩放比例分别保存</small></span>
      <input
        type="checkbox"
        :checked="settings.rememberPosition"
        :disabled="saving"
        @change="update({ rememberPosition: ($event.target as HTMLInputElement).checked })"
      />
    </label>
    <button class="refresh-now" :disabled="saving" @click="emit('refresh')">
      立即刷新全部账号
    </button>
    <p>按住圆球拖动；摘要展开后仍可拖动左侧圆球。</p>
  </section>
</template>

<style scoped>
.settings-view { flex: 1; min-height: 0; padding: 10px 22px 18px; border-top: 1px solid rgba(145, 161, 183, .24); }
.setting-row { min-height: 64px; display: flex; align-items: center; justify-content: space-between; gap: 20px; border-bottom: 1px solid rgba(145, 161, 183, .15); }
.setting-row > span { display: grid; gap: 5px; }
strong { color: #eef4fc; font-size: 13px; font-weight: 650; }
small, p { color: #8997aa; font-size: 11px; }
select { min-width: 108px; color: #e6edf7; background: #1d2837; border: 1px solid #43536a; border-radius: 8px; padding: 7px 9px; outline: none; }
input[type="checkbox"] { width: 34px; height: 18px; accent-color: #168dff; cursor: pointer; }
.refresh-now { display: block; margin: 17px auto 0; padding: 8px 18px; border: 1px solid #1685ff; border-radius: 8px; color: white; background: #087ef5; cursor: pointer; }
.refresh-now:hover { background: #2392ff; }
.refresh-now:disabled, select:disabled, input:disabled { opacity: .55; cursor: default; }
p { margin: 12px 0 0; text-align: center; }
</style>
