//! Naming the program a process runs, which the window module's class is
//! shown by: [`name`] on every platform's tests, [`windows::path`] reading a
//! process's on Windows.

#[cfg(windows)]
pub mod windows;

/// A program's name from its path: its file name without `.exe`.
#[must_use]
pub fn name(path: &str) -> &str {
    // Split on both separators, so the tests run the same everywhere.
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let stem = file.len().checked_sub(".exe".len()).filter(|&end| {
        file.get(end..)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(".exe"))
    });
    stem.map_or(file, |end| &file[..end])
}

#[cfg(test)]
mod tests {
    use super::name;

    #[test]
    fn a_program_is_named_by_its_file_without_exe() {
        assert_eq!(
            name("C:\\Program Files\\Mozilla Firefox\\firefox.EXE"),
            "firefox"
        );
        assert_eq!(name("C:\\x\\Spotify.exe"), "Spotify");
        assert_eq!(name("/opt/tool"), "tool");
        assert_eq!(name("exe"), "exe", "shorter than the extension");
    }
}
