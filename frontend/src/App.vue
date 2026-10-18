<script setup lang="ts">
import { computed } from 'vue'
import GpuCard from './components/GpuCard.vue'
import { useGpuStream } from './composables/useGpuStream'

const WINDOW_SEC = 300
const { snapshot, connected, series } = useGpuStream(WINDOW_SEC)

const summary = computed(() => {
  const gpus = snapshot.value?.gpus ?? []
  if (gpus.length === 0) return null
  const powers = gpus.map((g) => g.powerW).filter((v): v is number => v != null)
  const temps = gpus.map((g) => g.tempC).filter((v): v is number => v != null)
  const utils = gpus.map((g) => g.utilGpu).filter((v): v is number => v != null)
  return {
    count: gpus.length,
    totalPower: powers.reduce((a, b) => a + b, 0),
    maxTemp: temps.length ? Math.max(...temps) : null,
    avgTemp: temps.length ? temps.reduce((a, b) => a + b, 0) / temps.length : null,
    maxUtil: utils.length ? Math.max(...utils) : null,
  }
})
</script>

<template>
  <main class="mx-auto max-w-6xl px-4 py-6 sm:px-6">
    <!-- top bar -->
    <header class="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 class="text-xl font-bold tracking-tight text-slate-100">
          GPU<span class="text-util">MON</span>
        </h1>
        <p class="font-mono text-[11px] text-slate-500">realtime · nvml · 1s sampling</p>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <!-- connection pill -->
        <span
          class="flex items-center gap-1.5 rounded-full border px-3 py-1 font-mono text-[11px]"
          :class="connected ? 'border-util/40 bg-util/10 text-util' : 'border-[#ff4d4f]/40 bg-[#ff4d4f]/10 text-[#ff7875]'"
        >
          <span class="size-1.5 rounded-full" :class="connected ? 'bg-util' : 'bg-[#ff7875] animate-pulse'" />
          {{ connected ? 'LIVE' : 'OFFLINE' }}
        </span>
        <!-- summary stats -->
        <template v-if="summary">
          <span class="rounded-full border border-edge bg-panel px-3 py-1 font-mono text-[11px] text-slate-400">
            {{ summary.count }} GPU{{ summary.count > 1 ? 'S' : '' }}
          </span>
          <span class="rounded-full border border-edge bg-panel px-3 py-1 font-mono text-[11px]">
            <span class="text-slate-500">PWR </span><span class="text-power">{{ summary.totalPower.toFixed(0) }} W</span>
          </span>
          <span class="rounded-full border border-edge bg-panel px-3 py-1 font-mono text-[11px]">
            <span class="text-slate-500">TEMP </span><span class="text-temp">{{ summary.maxTemp?.toFixed(0) ?? '--' }}°C</span>
            <span class="text-slate-600"> avg {{ summary.avgTemp?.toFixed(0) ?? '--' }}</span>
          </span>
          <span class="rounded-full border border-edge bg-panel px-3 py-1 font-mono text-[11px]">
            <span class="text-slate-500">UTIL </span><span class="text-util">{{ summary.maxUtil?.toFixed(0) ?? '--' }}%</span>
          </span>
        </template>
      </div>
    </header>

    <!-- offline banner -->
    <div v-if="!connected"
         class="mb-5 rounded-xl border border-[#ff4d4f]/30 bg-[#ff4d4f]/5 px-4 py-3 font-mono text-xs text-[#ff7875]">
      数据已断开，正在自动重连…
    </div>

    <!-- loading -->
    <div v-if="!snapshot" class="grid place-items-center py-24 text-slate-600">
      <p class="animate-pulse font-mono text-sm">waiting for stream…</p>
    </div>

    <!-- gpu cards -->
    <div v-else class="grid grid-cols-1 gap-4 2xl:grid-cols-1">
      <GpuCard
        v-for="gpu in snapshot.gpus"
        :key="gpu.index"
        :gpu="gpu"
        :series="series.get(gpu.index)"
      />
    </div>

    <footer v-if="snapshot && snapshot.errorCount > 0"
            class="mt-6 text-center font-mono text-[11px] text-slate-600">
      sampling errors: {{ snapshot.errorCount }}
    </footer>
  </main>
</template>
