pub(crate) fn wait_status_message(elapsed_secs: u64) -> String {
    let stage = match elapsed_secs {
        0..=30 => "本地后端正在生成",
        31..=120 => "本地后端仍在运行，真实 Gemma 长回答或首次缓存可能较慢",
        _ => "本地后端仍在运行，请保持页面打开，除非出现错误",
    };
    format!("{stage}（已等待 {elapsed_secs}s）")
}

pub(crate) fn backend_error_hint(error: &str) -> String {
    if error.contains("10054") || error.contains("强迫关闭") {
        format!("{error}；后端在模型推理期间关闭了连接，请检查 rust-norion 和 mistralrs 进程")
    } else if error.contains("connect backend failed") {
        format!("{error}；当前配置的 rust-norion 后端地址不可达")
    } else {
        error.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_status_describes_long_local_inference() {
        assert!(wait_status_message(4).contains("本地后端正在生成"));
        assert!(wait_status_message(62).contains("长回答"));
        assert!(wait_status_message(180).contains("保持页面打开"));
    }

    #[test]
    fn backend_error_hint_explains_forced_close() {
        let hint = backend_error_hint("read backend response failed: os error 10054");
        assert!(hint.contains("后端在模型推理期间关闭了连接"));
    }
}
