//! Hold an OS lock for the lifetime of the graphics application.
use std::{fs::File, path::Path};

pub fn acquire(path: &Path) -> Result<File, String> {
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("Cannot open PHALANX instance lock: {e}"))?;
    file.try_lock().map_err(|e| format!("PHALANX is already running or its instance lock is unavailable: {e}. Close the existing game before launching again."))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    #[test]
    fn another_instance_is_rejected_and_exit_releases_the_lock() {
        let path = std::env::temp_dir().join(format!("phalanx-lock-test-{}", std::process::id()));
        let first = super::acquire(&path).unwrap();
        assert!(super::acquire(&path).is_err());
        drop(first);
        let second = super::acquire(&path).unwrap();
        drop(second);
        std::fs::remove_file(path).unwrap();
    }
}
