#![no_main]

use libfuzzer_sys::fuzz_target;
use sy_api::persistence::IEventLog;
use sy_infra::store::FileEventLog;

fuzz_target!(|data: &[u8]| {
    let dir = tempfile::tempdir().expect("tempdir must be available for WAL fuzzing");
    let path = dir.path().join("events.wal");

    if std::fs::write(&path, data).is_err() {
        return;
    }

    if let Ok(log) = FileEventLog::new(&path) {
        let _ = log.read_all_valid();
    }
});
