<script setup lang="ts">
import { computed } from 'vue'
import type { GpuInfo } from '../types'
import type { Series } from '../composables/useGpuStream'
import SparkChart from './SparkChart.vue'

const props = defineProps<{ gpu: GpuInfo; series?: Series }>()

const fmt = (v: number | null | undefined, digits = 0) =>
  v == null ? '--' : v.toFixed(digits)

const memPct = computed(() =>
  props.gpu.memTotalMb > 0 ? (props.gpu.memUsedMb / props.gpu.memTotalMb) * 100 : 0,
)

const tempHot = computed(
  () => props.gpu.tempC != null && props.gpu.shutdownTempC != null
    ? props.gpu.tempC >= props.gpu.shutdownTempC - 15
    : false,
)

const powerPct = computed(() =>
  props.gpu.powerW != null && props.gpu.powerLimitW
    ? (props.gpu.powerW / props.gpu.powerLimitW) * 100
    : 0,
)

function barColor(pct: number) {
  if (pct > 85) return 'bg-[#ff4d4f]'
  if (pct > 60) return 'bg-power'
  return 'bg-util'
}
</script>

<template>
  <section class="reveal rounded-2xl border border-edge bg-panel p-4 shadow-[0_8px_30px_rgba(0,0,0,0.35)] sm:p-5">
    <!-- header -->
    <header class="mb-4 flex flex-wrap items-center justify-between gap-2">
      <div class="flex items-center gap-3">
        <span class="grid size-8 place-items-center rounded-lg bg-util/10 font-mono text-sm font-extrabold text-util">
          {{ gpu.index }}
        </span>
        <div>
          <h2 class="text-sm font-bold tracking-wide text-slate-100">{{ gpu.name }}</h2>
          <p class="font-mono text-[11px] text-slate-500">
            <span v-if="gpu.memTempC != null">vram {{ fmt(gpu.memTempC) }}°C · </span>core {{ fmt(gpu.coreClockMhz) }} MHz · mem {{ fmt(gpu.memClockMhz) }} MHz
            <span v-if="gpu.fanSpeedPct != null"> · fan {{ fmt(gpu.fanSpeedPct) }}%</span>
          </p>
        </div>
      </div>
      <div
        class="rounded-full border px-3 py-1 font-mono text-lg font-extrabold"
        :class="tempHot
          ? 'border-[#ff4d4f]/50 bg-[#ff4d4f]/10 text-[#ff7875] animate-pulse'
          : 'border-edge bg-panel-deep text-temp'"
      >
        {{ fmt(gpu.tempC) }}°C
        <span v-if="gpu.shutdownTempC" class="ml-1 text-[10px] font-normal text-slate-500">
          / {{ gpu.shutdownTempC }}°C
        </span>
      </div>
    </header>

    <!-- bars -->
    <div class="mb-4 grid grid-cols-2 gap-x-5 gap-y-3">
      <div>
        <div class="mb-1 flex justify-between font-mono text-[11px]">
          <span class="text-slate-500">GPU</span>
          <span class="text-util">{{ fmt(gpu.utilGpu) }}%</span>
        </div>
        <div class="h-1.5 overflow-hidden rounded-full bg-panel-deep">
          <div class="h-full rounded-full bg-util transition-all duration-500"
               :style="{ width: `${gpu.utilGpu ?? 0}%` }" />
        </div>
      </div>
      <div>
        <div class="mb-1 flex justify-between font-mono text-[11px]">
          <span class="text-slate-500">POWER</span>
          <span class="text-power">{{ fmt(gpu.powerW, 1) }} / {{ fmt(gpu.powerLimitW) }} W</span>
        </div>
        <div class="h-1.5 overflow-hidden rounded-full bg-panel-deep">
          <div class="h-full rounded-full transition-all duration-500"
               :class="barColor(powerPct)" :style="{ width: `${Math.min(powerPct, 100)}%` }" />
        </div>
      </div>
      <div>
        <div class="mb-1 flex justify-between font-mono text-[11px]">
          <span class="text-slate-500">VRAM</span>
          <span class="text-mem">
            {{ (gpu.memUsedMb / 1024).toFixed(1) }} / {{ (gpu.memTotalMb / 1024).toFixed(0) }} GiB
          </span>
        </div>
        <div class="h-1.5 overflow-hidden rounded-full bg-panel-deep">
          <div class="h-full rounded-full bg-mem transition-all duration-500"
               :style="{ width: `${memPct}%` }" />
        </div>
      </div>
      <div>
        <div class="mb-1 flex justify-between font-mono text-[11px]">
          <span class="text-slate-500">MEM UTIL</span>
          <span class="text-slate-300">{{ fmt(gpu.utilMem) }}%</span>
        </div>
        <div class="h-1.5 overflow-hidden rounded-full bg-panel-deep">
          <div class="h-full rounded-full bg-slate-400 transition-all duration-500"
               :style="{ width: `${gpu.utilMem ?? 0}%` }" />
        </div>
      </div>
    </div>

    <!-- charts -->
    <div v-if="series" class="grid grid-cols-1 gap-x-4 gap-y-2 sm:grid-cols-3">
      <SparkChart label="TEMP °C" unit="°C" color="#ff7a45" :times="series.t" :values="series.temp" />
      <SparkChart label="POWER W" unit="W" color="#ffc53d" :times="series.t" :values="series.power" />
      <SparkChart label="UTIL %" unit="%" color="#36cfc9" :times="series.t" :values="series.util" />
    </div>

    <!-- processes -->
    <div class="mt-4 border-t border-edge pt-3">
      <p v-if="gpu.procs.length === 0" class="font-mono text-[11px] text-slate-600">idle · no processes</p>
      <ul v-else class="grid grid-cols-1 gap-1 sm:grid-cols-2">
        <li v-for="p in gpu.procs.slice(0, 8)" :key="p.pid"
            class="flex items-baseline justify-between font-mono text-[11px] text-slate-400">
          <span class="truncate">{{ p.name || 'unknown' }} <span class="text-slate-600">#{{ p.pid }}</span></span>
          <span class="text-mem">{{ (p.memUsedMb / 1024).toFixed(2) }} GiB</span>
        </li>
      </ul>
    </div>
  </section>
</template>
