use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor, TemperatureThreshold};
use nvml_wrapper::enums::device::UsedGpuMemory;
use nvml_wrapper::error::NvmlError;
use nvml_wrapper::Nvml;
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuProcess {
    pub pid: u32,
    pub name: String,
    pub mem_used_mb: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub temp_c: Option<i32>,
    pub shutdown_temp_c: Option<i32>,
    pub util_gpu: Option<u32>,
    pub util_mem: Option<u32>,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub power_w: Option<f32>,
    pub power_limit_w: Option<f32>,
    pub fan_speed_pct: Option<u32>,
    pub core_clock_mhz: Option<u32>,
    pub mem_clock_mhz: Option<u32>,
    pub procs: Vec<GpuProcess>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub ts: u64,
    pub error_count: u64,
    pub gpus: Vec<GpuInfo>,
}

/// Read one sample from every GPU. Per-field failures degrade to `None`
/// (unsupported sensors on e.g. CMP cards) and bump `errors`.
pub fn sample(nvml: &Nvml, errors: &mut u64) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let count = match nvml.device_count() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("device_count failed: {e}");
            *errors += 1;
            return gpus;
        }
    };
    for i in 0..count {
        let dev = match nvml.device_by_index(i) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("device_by_index({i}) failed: {e}");
                *errors += 1;
                continue;
            }
        };
        // Unsupported sensors (fans etc.) degrade to None silently;
        // only unexpected failures are counted.
        let mut get = |r: Result<u32, NvmlError>| match r {
            Ok(v) => Some(v),
            Err(NvmlError::NotSupported) | Err(NvmlError::InsufficientSize(_)) => None,
            Err(_) => {
                *errors += 1;
                None
            }
        };
        let name = dev.name().unwrap_or_else(|_| "unknown".into());
        let mem = dev
            .memory_info()
            .map(|m| (m.used / 1024 / 1024, m.total / 1024 / 1024))
            .unwrap_or((0, 0));
        let util = dev.utilization_rates().map(|u| (u.gpu, u.memory)).ok();
        // Compute-type lookup; graphics-only processes are not covered in v1.
        let procs = match dev.running_compute_processes() {
            Ok(list) => list
                .into_iter()
                .map(|p| GpuProcess {
                    pid: p.pid,
                    name: proc_name(p.pid),
                    mem_used_mb: match p.used_gpu_memory {
                        UsedGpuMemory::Used(bytes) => bytes / 1024 / 1024,
                        UsedGpuMemory::Unavailable => 0,
                    },
                })
                .collect(),
            Err(_) => Vec::new(), // not supported on all driver lines
        };
        gpus.push(GpuInfo {
            index: i,
            name,
            temp_c: get(dev.temperature(TemperatureSensor::Gpu)).map(|t| t as i32),
            // NOTE: HBM temperature needs NVML field values; nvml-wrapper 0.10
            // does not expose it. Skipped for now (CMP cards lack it anyway).
            shutdown_temp_c: get(dev.temperature_threshold(TemperatureThreshold::Shutdown))
                .map(|t| t as i32),
            util_gpu: util.map(|u| u.0),
            util_mem: util.map(|u| u.1),
            mem_used_mb: mem.0,
            mem_total_mb: mem.1,
            power_w: get(dev.power_usage()).map(|mw| mw as f32 / 1000.0),
            power_limit_w: get(dev.power_management_limit()).map(|mw| mw as f32 / 1000.0),
            fan_speed_pct: dev.fan_speed(0).ok().filter(|v| *v != u32::MAX),
            core_clock_mhz: get(dev.clock_info(Clock::Graphics)),
            mem_clock_mhz: get(dev.clock_info(Clock::Memory)),
            procs,
        });
    }
    gpus
}

/// Best-effort process name from procfs (NVML v2 calls do not return names).
fn proc_name(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = Snapshot {
            ts: 123,
            error_count: 0,
            gpus: vec![GpuInfo {
                index: 0,
                name: "test".into(),
                temp_c: Some(42),
                shutdown_temp_c: Some(98),
                util_gpu: Some(10),
                util_mem: Some(20),
                mem_used_mb: 1,
                mem_total_mb: 2,
                power_w: Some(3.5),
                power_limit_w: Some(250.0),
                fan_speed_pct: None,
                core_clock_mhz: Some(1000),
                mem_clock_mhz: Some(2000),
                procs: vec![GpuProcess {
                    pid: 9,
                    name: "x".into(),
                    mem_used_mb: 5,
                }],
            }],
        };
        let json = serde_json::to_string(&snap).unwrap();
        // Field names must match the frontend types.ts contract.
        for key in [
            "tempC",
            "shutdownTempC",
            "utilGpu",
            "memUsedMb",
            "powerW",
            "powerLimitW",
            "fanSpeedPct",
            "coreClockMhz",
            "memClockMhz",
            "memUsedMb",
            "errorCount",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        assert!(json.contains("\"fanSpeedPct\":null"));
    }
}
