# gpu-monitor

Realtime NVIDIA GPU web monitor. Rust (axum + NVML) backend, Vue 3 + Tailwind + ECharts frontend,
embedded into a single binary.

## Build & run

```bash
cd frontend && npm install && npm run build && cd ..
cargo run --release                 # http://localhost:8080
```

Options: `--port 8080` `--interval 1000` (ms, min 250) `--window 300` (s).

## Development (hot reload on both sides)

```bash
# terminal 1
cargo run
# terminal 2
cd frontend && npm run dev          # http://localhost:5173, proxies /ws to :8080
```
