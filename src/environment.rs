use std::{env::consts::OS, sync::LazyLock};

pub struct EnvironmentInfo {
    paltform: &'static str,
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        Self { paltform: OS }
    }
}

pub static CACHED_ENVIRONMENT_INFO: LazyLock<EnvironmentInfo> =
    LazyLock::new(|| EnvironmentInfo { paltform: OS });
