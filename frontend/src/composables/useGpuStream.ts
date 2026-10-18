import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { Snapshot } from '../types'

export interface Series {
  t: number[]
  temp: (number | null)[]
  power: (number | null)[]
  util: (number | null)[]
}

import type { Ref } from 'vue'

export function useGpuStream(windowSec: Ref<number>) {
  const snapshot = ref<Snapshot | null>(null)
  const connected = ref(false)
  const series = ref<Map<number, Series>>(new Map())
  let ws: WebSocket | null = null
  let reconnectTimer: number | undefined
  let closed = false

  function connect() {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws'
    ws = new WebSocket(`${proto}://${location.host}/ws`)
    ws.onopen = () => (connected.value = true)
    ws.onmessage = (ev) => {
      const snap = JSON.parse(ev.data) as Snapshot
      snapshot.value = snap
      // Adopt the server-advertised window (config: --window).
      if (snap.windowSec > 0) windowSec.value = snap.windowSec
      const cutoff = snap.ts - windowSec.value * 1000
      // Copy-on-write: new arrays/objects per frame so child watchers fire.
      const next = new Map(series.value)
      for (const gpu of snap.gpus) {
        const prev = next.get(gpu.index) ?? { t: [], temp: [], power: [], util: [] }
        const cur: Series = {
          t: [...prev.t, snap.ts],
          temp: [...prev.temp, gpu.tempC],
          power: [...prev.power, gpu.powerW],
          util: [...prev.util, gpu.utilGpu],
        }
        while (cur.t.length > 0 && cur.t[0] < cutoff) {
          cur.t.shift()
          cur.temp.shift()
          cur.power.shift()
          cur.util.shift()
        }
        next.set(gpu.index, cur)
      }
      series.value = next
    }
    ws.onclose = () => {
      connected.value = false
      if (!closed) reconnectTimer = window.setTimeout(connect, 2000)
    }
    ws.onerror = () => ws?.close()
  }

  onMounted(connect)
  onBeforeUnmount(() => {
    closed = true
    window.clearTimeout(reconnectTimer)
    ws?.close()
  })

  return { snapshot, connected, series }
}
