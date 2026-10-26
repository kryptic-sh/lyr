use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{data_dirs, icon_in};

fn entry(dir: &Path, id: &str, text: &str) {
    let applications = dir.join("applications");
    std::fs::create_dir_all(&applications).expect("created");
    std::fs::write(applications.join(format!("{id}.desktop")), text).expect("written");
}

#[test]
fn the_first_id_with_an_entry_gives_its_icon_the_user_s_first() {
    let (user, system) = (
        tempfile::tempdir().expect("a directory"),
        tempfile::tempdir().expect("a directory"),
    );
    entry(
        system.path(),
        "firefox",
        "[Desktop Entry]\nName=Firefox\nIcon=firefox\n",
    );
    entry(user.path(), "firefox", "[Desktop Entry]\nIcon=my-firefox\n");
    entry(
        system.path(),
        "discord",
        "[Desktop Entry]\nIcon=/opt/discord/icon.png\n",
    );
    entry(system.path(), "blank", "[Desktop Entry]\nIcon=\n");
    let dirs = [user.path().to_owned(), system.path().to_owned()];
    assert_eq!(
        icon_in(&dirs, ["Firefox", "firefox"]).as_deref(),
        Some("my-firefox"),
        "the first id that has one, from the user's directory"
    );
    assert_eq!(
        icon_in(&dirs, ["discord"]).as_deref(),
        Some("/opt/discord/icon.png")
    );
    assert_eq!(icon_in(&dirs, ["blank", "nothing"]), None);
}

#[test]
fn an_id_cannot_reach_outside_the_applications_directory() {
    let root = tempfile::tempdir().expect("a directory");
    let data = root.path().join("data");
    entry(&data, "x", "[Desktop Entry]\nIcon=x\n");
    // A desktop entry `../../outside` would reach from `data/applications`.
    std::fs::write(
        root.path().join("outside.desktop"),
        "[Desktop Entry]\nIcon=outside\n",
    )
    .expect("written");
    let dirs = [data];
    assert_eq!(icon_in(&dirs, ["x"]).as_deref(), Some("x"));
    for id in ["../../outside", ".hidden", "", "a/b"] {
        assert_eq!(icon_in(&dirs, [id]), None, "{id:?}");
    }
}

#[test]
fn the_data_directories_follow_the_base_directory_specification() {
    let env = |vars: &'static [(&'static str, &'static str)]| {
        move |name: &str| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    };
    assert_eq!(
        data_dirs(env(&[("HOME", "/home/u")])),
        [
            PathBuf::from("/home/u/.local/share"),
            PathBuf::from("/usr/local/share"),
            PathBuf::from("/usr/share"),
        ]
    );
    assert_eq!(
        data_dirs(env(&[
            ("HOME", "/home/u"),
            ("XDG_DATA_HOME", "/data"),
            ("XDG_DATA_DIRS", "/a::/b"),
        ])),
        [
            PathBuf::from("/data"),
            PathBuf::from("/a"),
            PathBuf::from("/b"),
        ]
    );
}
