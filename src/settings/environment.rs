use std::{env::consts::OS, sync::LazyLock};

pub struct EnvironmentInfo {
    pub platform: &'static str,
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        Self { platform: OS }
    }
}

pub static CACHED_ENVIRONMENT_INFO: LazyLock<EnvironmentInfo> =
    LazyLock::new(|| EnvironmentInfo { platform: OS });
