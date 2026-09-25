use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use taskboard_application::AppError;

/// Exclusive advisory lock on `{data_dir}/taskboard.lock`.
/// Dropping the guard releases the lock.
#[derive(Debug)]
pub struct DataLock {
    _file: File,
}

pub fn try_acquire_data_lock(data_dir: &Path) -> Result<DataLock, AppError> {
    fs::create_dir_all(data_dir).map_err(|err| AppError::Io(err.to_string()))?;
    let path = data_dir.join("taskboard.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|err| AppError::Io(err.to_string()))?;
    // LOCK_EX | LOCK_NB. Same-process second opens fail so import can see serve.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::WouldBlock {
            return Err(AppError::DatabaseInUse);
        }
        return Err(AppError::Io(err.to_string()));
    }
    Ok(DataLock { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_is_database_in_use() {
        let tmp = tempfile::tempdir().unwrap();
        let held = try_acquire_data_lock(tmp.path()).unwrap();
        let err = try_acquire_data_lock(tmp.path()).unwrap_err();
        assert_eq!(err.code(), "conflict");
        assert!(err.to_string().contains("desktop"));
        drop(held);
        try_acquire_data_lock(tmp.path()).unwrap();
    }
}
