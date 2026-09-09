<script setup lang="ts">
import { computed } from "vue";
import type { QuotaWindow } from "../types/account";

const props = defineProps<{
  fiveHour?: QuotaWindow;
  weekly?: QuotaWindow;
  codexRunning: boolean;
}>();

const circumference = (radius: number) => 2 * Math.PI * radius;
const dash = (value: number | undefined, radius: number) => {
  const total = circumference(radius);
  const percent = Math.min(100, Math.max(0, value ?? 0));
  return `${(total * percent) / 100} ${total}`;
};

const fiveDash = computed(() => dash(props.fiveHour?.remainingPercent, 27));
const weeklyDash = computed(() => dash(props.weekly?.remainingPercent, 34));
</script>

<template>
  <div class="quota-orb" title="拖动悬浮球；点击展开账号面板">
    <svg viewBox="0 0 76 76" aria-label="当前账号剩余额度">
      <circle class="ring-track" cx="38" cy="38" r="34" />
      <circle class="ring-track inner" cx="38" cy="38" r="27" />
      <circle class="ring weekly" cx="38" cy="38" r="34" :stroke-dasharray="weeklyDash" />
      <circle class="ring five" cx="38" cy="38" r="27" :stroke-dasharray="fiveDash" />
      <g class="knot-mark" aria-hidden="true">
        <rect v-for="angle in [0, 60, 120]" :key="angle" x="34" y="22" width="8" height="32" rx="4" :transform="`rotate(${angle} 38 38)`" />
      </g>
      <circle class="center-cut" cx="38" cy="38" r="8" />
      <circle class="process-dot-border" cx="51" cy="51" r="7" />
      <circle :class="['process-dot', { stopped: !codexRunning }]" cx="51" cy="51" r="5" />
    </svg>
  </div>
</template>

<style scoped>
.quota-orb {
  position: relative;
  width: 76px;
  height: 76px;
  border-radius: 50%;
  background: radial-gradient(circle at 35% 25%, #293343, #111722 68%);
  box-shadow: inset 0 0 0 1px rgba(192, 213, 240, 0.18), 0 8px 26px rgba(0, 0, 0, 0.42);
  animation: breathe 4s ease-in-out infinite;
  flex: 0 0 76px;
}
svg { display: block; width: 100%; height: 100%; overflow: visible; }
.ring-track, .ring {
  fill: none;
  stroke-width: 4;
  transform: rotate(-90deg);
  transform-origin: 38px 38px;
}
.ring-track { stroke: #30394a; }
.ring-track.inner { stroke: #293544; }
.ring { stroke-linecap: round; }
.ring.five { stroke: #4dd7ff; filter: drop-shadow(0 0 3px rgba(77, 215, 255, .4)); }
.ring.weekly { stroke: #9a7bff; filter: drop-shadow(0 0 3px rgba(154, 123, 255, .36)); }
.knot-mark rect { fill: none; stroke: #f4f8ff; stroke-width: 3.2; }
.center-cut { fill: #141b26; stroke: #f4f8ff; stroke-width: 2.4; }
.process-dot-border { fill: #161d28; }
.process-dot { fill: #42d392; filter: drop-shadow(0 0 3px rgba(66, 211, 146, .5)); }
.process-dot.stopped { fill: #77808e; filter: none; }
@keyframes breathe {
  0%, 100% { transform: scale(1); filter: drop-shadow(0 0 6px rgba(77, 215, 255, .08)); }
  50% { transform: scale(1.015); filter: drop-shadow(0 0 10px rgba(126, 124, 255, .14)); }
}
@media (prefers-reduced-motion: reduce) { .quota-orb { animation: none; } }
</style>
