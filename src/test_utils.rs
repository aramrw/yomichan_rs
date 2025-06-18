use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, RwLock},
};
use tempfile::{tempdir_in, TempDir};

use crate::{database::dictionary_database::DictionaryDatabase, Yomichan};

pub(crate) struct TestPaths {
    pub tests_dir: PathBuf,
    pub tests_yomichan_db_path: PathBuf,
    pub test_dicts_dir: PathBuf,
}

pub(crate) static TEST_PATHS: LazyLock<TestPaths> = LazyLock::new(|| TestPaths {
    tests_dir: PathBuf::from("./tests"),
    tests_yomichan_db_path: PathBuf::from("./tests").join("yomichan_rs").join("db.ycd"),
    test_dicts_dir: PathBuf::from("tests").join("test_dicts"),
});

pub static YCD: LazyLock<RwLock<Yomichan>> = LazyLock::new(|| {
    let mut ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
    ycd.set_language("es");
    RwLock::new(ycd)
});

pub(crate) static SHARED_DB_INSTANCE: LazyLock<DictionaryDatabase> = LazyLock::new(|| {
    let db_path = &*TEST_PATHS.tests_yomichan_db_path;

    if !db_path.exists() {
        panic!("SHARED_DB_INSTANCE: The database path for tests does not exist: {db_path:?}");
    }

    // Create the DictionaryDatabase instance once.
    // This instance (and its underlying native_db::Database connection)
    // will be shared by all tests that use it.
    DictionaryDatabase::new(db_path)
});

pub(crate) enum BacktraceKind {
    One,
    Full,
}

impl std::fmt::Display for BacktraceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = match self {
            BacktraceKind::One => "1",
            BacktraceKind::Full => "full",
        };
        write!(f, "{x}")
    }
}

pub(crate) fn set_backtrace(kind: BacktraceKind) {
    std::env::set_var("RUST_BACKTRACE", kind.to_string());
}

/// Copies the test database to a temporary directory.
/// Necessary because native_db cannot have two test threads
/// with different Database connections open the same file at the same time.
pub(crate) fn copy_test_db() -> (PathBuf, TempDir) {
    let dir = tempdir_in(&*TEST_PATHS.tests_dir).unwrap();
    let tydbp = &*TEST_PATHS.tests_yomichan_db_path;
    let f_path = dir.path().join("data.ycd");
    if !tydbp.exists() {
        panic!("tests/yomichan_db_path doesn't exist! : {tydbp:?}");
    }
    std::fs::copy(tydbp, &f_path).unwrap();
    if !f_path.exists() {
        panic!("path doesn't exist! : {f_path:?}");
    }
    (f_path, dir)
}

pub(crate) fn print_timer<T>(inst: std::time::Instant, print: T)
where
    T: std::fmt::Debug,
{
    let duration = inst.elapsed();
    #[allow(unused_assignments)]
    let mut time = String::new();
    {
        let dur_sec = duration.as_secs();
        let dur_mill = duration.as_millis();
        let dur_nan = duration.as_nanos();
        if dur_sec == 0 {
            if dur_mill == 0 {
                time = format!("{dur_mill}ns");
            } else {
                time = format!("{dur_nan}ms");
            }
        } else if dur_sec > 60 {
            let min = dur_sec / 60;
            let sec = dur_sec % 60;
            time = format!("{min}m{sec}s");
        } else {
            time = format!("{dur_sec}s");
        }
    }
    println!("{print:?} files");
    println!("in {time}");
}
