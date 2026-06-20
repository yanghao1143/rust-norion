use super::DeviceClass;

pub(super) fn parse_device_class(value: &str) -> Result<DeviceClass, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(DeviceClass::Auto),
        "cpu" | "cpu-only" | "cpu_only" | "pc-cpu" | "desktop-cpu" | "generic" | "fallback"
        | "unknown" | "unknown-device" | "x86" | "x86_64" | "amd64" | "arm64" | "aarch64"
        | "loongarch64" | "avx2" | "avx512" | "sse4" | "neon" | "portable" => {
            Ok(DeviceClass::CpuOnly)
        }
        "integrated"
        | "igpu"
        | "integrated-gpu"
        | "laptop"
        | "notebook"
        | "intel-gpu"
        | "intel-iris"
        | "intel-xe"
        | "intel-uhd"
        | "intel-hd"
        | "amd-apu"
        | "apu"
        | "amd-radeon-graphics"
        | "rdna-apu"
        | "ultrabook"
        | "mini-pc"
        | "handheld-pc"
        | "steamdeck"
        | "handheld-console"
        | "portable-console" => Ok(DeviceClass::IntegratedGpu),
        "discrete" | "dgpu" | "discrete-gpu" | "desktop-gpu" | "gpu" | "cuda" | "rtx"
        | "nvidia" | "nvidia-gpu" | "radeon" | "amd-gpu" | "arc" | "intel-arc" | "vulkan-gpu"
        | "opencl" | "directml" | "dml" | "egpu" => Ok(DeviceClass::DiscreteGpu),
        "uma" | "unified" | "unified-memory" | "apple" | "mac" | "macbook" | "m-series"
        | "apple-silicon" | "m1" | "m2" | "m3" | "m4" | "m5" => Ok(DeviceClass::UnifiedMemory),
        "mobile" | "phone" | "tablet" | "android" | "ios" | "handheld" | "iphone" | "ipad"
        | "harmonyos" | "ohos" | "visionos" | "smartphone" | "wearable" | "wear-os" | "wearos"
        | "watch" | "xr" | "vr" | "ar" | "quest" | "mobile-vr" | "smart-tv" | "tvos"
        | "android-tv" => Ok(DeviceClass::Mobile),
        "embedded"
        | "iot"
        | "rpi"
        | "raspberry-pi"
        | "raspberry_pi"
        | "sbc"
        | "arm-sbc"
        | "linux-sbc"
        | "single-board"
        | "single-board-computer"
        | "riscv"
        | "riscv64"
        | "risc-v"
        | "armv7"
        | "armv8"
        | "embedded-linux"
        | "yocto" => Ok(DeviceClass::Embedded),
        "browser-wasm"
        | "browser_wasm"
        | "wasm"
        | "wasm32"
        | "wasm32-wasip1"
        | "wasm32-unknown-unknown"
        | "wasi"
        | "wasip1"
        | "browser"
        | "web"
        | "web-runtime"
        | "webworker"
        | "service-worker"
        | "webgpu" => Ok(DeviceClass::BrowserWasm),
        "microcontroller" | "micro" | "mcu" | "tiny" | "tiny-device" | "no-std" | "cortex-m"
        | "thumbv7" | "thumbv8" | "xtensa" | "esp32" | "esp-idf" | "stm32" | "arduino"
        | "rp2040" | "riscv32" => Ok(DeviceClass::Microcontroller),
        "npu"
        | "ane"
        | "tpu"
        | "ai-accelerator"
        | "ai_accelerator"
        | "npu-accelerator"
        | "neural"
        | "neural-engine"
        | "snapdragon"
        | "qualcomm"
        | "hexagon"
        | "qnn-htp"
        | "apple-neural-engine"
        | "ascend"
        | "cann"
        | "cambricon"
        | "mlu"
        | "kunlun"
        | "sophgo"
        | "bm1684"
        | "rockchip-npu"
        | "rknn"
        | "horizon-bpu"
        | "hailo"
        | "ethos"
        | "directml-npu"
        | "vitis-ai"
        | "npu-smi"
        | "mediatek-apu" => Ok(DeviceClass::NpuAccelerator),
        "multi-gpu" | "multi_gpu" | "multi" | "multi-accelerator" | "multi-accel" | "multi-npu"
        | "distributed" | "multi-device" | "heterogeneous" | "cluster" | "nvlink"
        | "tensor-parallel" | "pipeline-parallel" | "mpi" | "slurm-cluster" => {
            Ok(DeviceClass::MultiGpu)
        }
        "edge" | "gateway" | "edge-gateway" | "jetson" | "nas" | "home-server" | "router"
        | "industrial-pc" | "ipc" | "robot" | "robotics" | "drone" | "vehicle" | "automotive"
        | "car" | "camera" | "nvr" | "edge-box" | "smart-camera" => Ok(DeviceClass::Edge),
        "server" | "workstation" | "rack" | "datacenter" | "local-cloud" | "hpc" | "k8s"
        | "hpc-node" | "kubernetes" | "bare-metal" | "cloud-host" | "epyc" | "xeon"
        | "threadripper" | "rackmount" | "slurm" | "pbs" => Ok(DeviceClass::Server),
        other => Err(format!("unknown device class: {other}")),
    }
}
