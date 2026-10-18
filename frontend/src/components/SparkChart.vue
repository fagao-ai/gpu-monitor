<script setup lang="ts">
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer])

const props = defineProps<{
  label: string
  unit: string
  color: string
  times: number[]
  values: (number | null)[]
}>()

const el = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null
let ro: ResizeObserver | null = null

function render() {
  if (!chart) return
  chart.setOption({
    animation: false,
    grid: { left: 2, right: 2, top: 6, bottom: 2, containLabel: false },
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#101418',
      borderColor: '#1e262e',
      textStyle: { color: '#dbe4ec', fontSize: 11 },
      valueFormatter: (v: number | null) => (v == null ? '-' : `${v} ${props.unit}`),
    },
    xAxis: {
      type: 'time',
      show: false,
      axisPointer: { type: 'line', lineStyle: { color: '#2c3944' } },
    },
    yAxis: {
      type: 'value',
      show: false,
      scale: true,
    },
    series: [
      {
        type: 'line',
        data: props.times.map((t, i) => [t, props.values[i]]),
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 1.5, color: props.color },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: props.color + '55' },
            { offset: 1, color: props.color + '00' },
          ]),
        },
      },
    ],
  })
}

onMounted(() => {
  chart = echarts.init(el.value!, undefined, { renderer: 'canvas' })
  render()
  ro = new ResizeObserver(() => chart?.resize())
  ro.observe(el.value!)
})

watch(() => [props.times, props.values], render, { deep: false })

onBeforeUnmount(() => {
  ro?.disconnect()
  chart?.dispose()
})
</script>

<template>
  <div class="min-w-0">
    <div class="mb-1 flex items-baseline justify-between">
      <span class="text-[10px] uppercase tracking-widest text-slate-500">{{ label }}</span>
    </div>
    <div ref="el" class="h-16 w-full" />
  </div>
</template>
