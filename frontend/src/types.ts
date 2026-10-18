export interface GpuProcess {
  pid: number
  name: string
  memUsedMb: number
}

export interface GpuInfo {
  index: number
  name: string
  tempC: number | null
  memTempC: number | null
  shutdownTempC: number | null
  utilGpu: number | null
  utilMem: number | null
  memUsedMb: number
  memTotalMb: number
  powerW: number | null
  powerLimitW: number | null
  fanSpeedPct: number | null
  coreClockMhz: number | null
  memClockMhz: number | null
  procs: GpuProcess[]
}

export interface Snapshot {
  ts: number
  errorCount: number
  gpus: GpuInfo[]
}
