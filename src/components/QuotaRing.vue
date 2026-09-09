<script setup lang="ts">
import { computed } from "vue";
import type { ProcessState, QuotaWindow } from "../types/account";

const props = defineProps<{
  fiveHour?: QuotaWindow;
  weekly?: QuotaWindow;
  processState: ProcessState;
  interactionHint?: string;
}>();

const circumference = (radius: number) => 2 * Math.PI * radius;
const dash = (value: number | undefined, radius: number) => {
  const total = circumference(radius);
  const percent = Math.min(100, Math.max(0, value ?? 0));
  return `${(total * percent) / 100} ${total}`;
};

const fiveDash = computed(() => dash(props.fiveHour?.remainingPercent, 27));
const weeklyDash = computed(() => dash(props.weekly?.remainingPercent, 34));
const statusText = computed(() => ({
  running: "Codex Desktop 正在运行",
  stopped: "Codex Desktop 未运行",
  switching: "正在切换 Codex 账号",
  error: "Codex 账号或进程状态异常",
})[props.processState]);
</script>

<template>
  <div class="quota-orb" :title="`${statusText}；${interactionHint || '按住并拖动悬浮球，单击展开'}`">
    <svg viewBox="0 0 76 76" aria-label="当前账号剩余额度">
      <circle class="ring-track" cx="38" cy="38" r="34" />
      <circle class="ring-track inner" cx="38" cy="38" r="27" />
      <circle class="ring weekly" cx="38" cy="38" r="34" :stroke-dasharray="weeklyDash" />
      <circle class="ring five" cx="38" cy="38" r="27" :stroke-dasharray="fiveDash" />
      <g class="chatgpt-mark" transform="translate(24 24) scale(1.1666667)" aria-hidden="true">
        <path d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z" />
      </g>
      <circle class="process-dot-border" cx="51" cy="51" r="7" />
      <circle :class="['process-dot', processState]" cx="51" cy="51" r="5" />
    </svg>
  </div>
</template>

<style scoped>
.quota-orb {
  position: relative;
  width: 76px;
  height: 76px;
  flex: 0 0 76px;
  border-radius: 50%;
  background: radial-gradient(circle at 36% 28%, #263143 0, #151d29 56%, #0b111a 100%);
  box-shadow: inset 0 0 0 1px rgba(206, 223, 246, .18), 0 7px 19px rgba(0, 0, 0, .48);
  animation: orb-breathe 4s ease-in-out infinite;
  isolation: isolate;
}
.quota-orb::before {
  content: "";
  position: absolute;
  inset: -2px;
  z-index: -1;
  border-radius: inherit;
  box-shadow: 0 0 9px rgba(83, 210, 255, .16), 0 0 15px rgba(146, 105, 255, .1);
  animation: aura-breathe 4s ease-in-out infinite;
}
svg { display: block; position: relative; z-index: 1; width: 100%; height: 100%; overflow: visible; }
.ring-track, .ring {
  fill: none;
  stroke-width: 4;
  transform: rotate(-90deg);
  transform-origin: 38px 38px;
}
.ring-track { stroke: #30394a; }
.ring-track.inner { stroke: #293544; }
.ring { stroke-linecap: round; transition: stroke-dasharray .55s ease-out; }
.ring.five { stroke: #4dd7ff; filter: drop-shadow(0 0 2px rgba(77, 215, 255, .42)); }
.ring.weekly { stroke: #9a7bff; filter: drop-shadow(0 0 2px rgba(154, 123, 255, .38)); }
.chatgpt-mark { fill: #f7f9fc; transform-origin: 38px 38px; animation: mark-breathe 4s ease-in-out infinite; }
.process-dot-border { fill: #121924; }
.process-dot { transition: fill .2s ease, opacity .2s ease; }
.process-dot.running { fill: #22d99a; animation: status-breathe 3.2s ease-in-out infinite; }
.process-dot.stopped { fill: #727d8c; }
.process-dot.switching { fill: #f4bf4f; animation: status-breathe 1.2s ease-in-out infinite; }
.process-dot.error { fill: #ff626d; }
@keyframes orb-breathe { 0%, 100% { transform: scale(1); } 50% { transform: scale(1.015); } }
@keyframes aura-breathe { 0%, 100% { opacity: .5; transform: scale(.99); } 50% { opacity: 1; transform: scale(1.02); } }
@keyframes mark-breathe { 0%, 100% { opacity: .94; } 50% { opacity: 1; } }
@keyframes status-breathe { 0%, 100% { opacity: .78; } 50% { opacity: 1; } }
@media (prefers-reduced-motion: reduce) {
  .quota-orb, .quota-orb::before, .chatgpt-mark, .process-dot { animation: none; }
}
</style>
