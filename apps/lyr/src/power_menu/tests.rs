use lyr_config::Platform;
use lyr_core::power_menu::{Entry, entries};

use super::configured;

#[test]
fn each_entry_reads_its_own_command() {
    let exec = |text: &str| {
        Some(lyr_config::Exec {
            exec: text.to_owned(),
        })
    };
    let menu = lyr_config::PowerMenu {
        lock: exec("lock"),
        sleep: exec("sleep"),
        hibernate: exec("hibernate"),
        shutdown: exec("shutdown"),
        logout: exec("logout"),
        reboot: exec("reboot"),
    };
    for entry in entries(Platform::Linux) {
        assert_eq!(
            configured(&menu, entry).map(|command| command.exec.as_str()),
            Some(entry.label().to_lowercase().as_str())
        );
    }
    assert_eq!(
        configured(&lyr_config::PowerMenu::default(), Entry::Lock),
        None
    );
}
