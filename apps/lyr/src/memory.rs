//! Keeping the bar's heap as small as what it holds
//! (`docs/plan/12-performance-and-power.md`): large buffers mapped on their
//! own, and the heap's free pages given back when a panel closes.

/// The size from which glibc maps an allocation on its own, unmapped when it
/// is freed: glibc's own default, `DEFAULT_MMAP_THRESHOLD_MIN`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
const MMAP_THRESHOLD: std::ffi::c_int = 128 * 1024;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
// SAFETY: both take plain integers and only change or release what glibc's
// allocator holds, so no argument can make them unsound; `mallopt` is
// called before the bar starts its threads.
unsafe extern "C" {
    safe fn mallopt(param: std::ffi::c_int, value: std::ffi::c_int) -> std::ffi::c_int;
    safe fn malloc_trim(pad: usize) -> std::ffi::c_int;
}

/// Fixes glibc's thresholds at their defaults, before the bar starts its
/// threads. Left alone, glibc raises them each time a mapped block is freed,
/// up to 32 MB, so a picture decoded or a file read once is kept in the heap
/// afterwards, where [`give_back`] cannot reach it in a thread's arena: a
/// 28 MB file read for a notification's image left 29 MB in the sources
/// thread's arena, 0.5 MB of it in use. Setting them stops the raising, as
/// `mallopt(3)` says.
///
/// Linux with glibc only; elsewhere it does nothing.
pub fn configure() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        /// glibc's `M_TRIM_THRESHOLD` and `M_MMAP_THRESHOLD` (`malloc.h`).
        const M_TRIM_THRESHOLD: std::ffi::c_int = -1;
        const M_MMAP_THRESHOLD: std::ffi::c_int = -3;
        for param in [M_TRIM_THRESHOLD, M_MMAP_THRESHOLD] {
            // Before logging starts, so to stderr, as `main`'s own warning.
            if mallopt(param, MMAP_THRESHOLD) == 0 {
                eprintln!("lyr: glibc refused mallopt({param}, {MMAP_THRESHOLD})");
            }
        }
    }
}

/// Returns the heap's free pages to the system. A panel's layout, glyphs and
/// buffers are freed when it closes, but glibc keeps the pages for later
/// allocations, so without this the bar would stay as large as it was with
/// its largest panel open.
///
/// Linux with glibc only, where it is `malloc_trim(0)`. Elsewhere it does
/// nothing, and whether Windows' or macOS's allocator holds on to a closed
/// panel's pages has not been measured.
pub fn give_back() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        // It answers whether any memory was released, which changes nothing.
        let _released = malloc_trim(0);
    }
}

#[cfg(all(test, target_os = "linux", target_env = "gnu"))]
mod tests {
    use std::sync::{Mutex, PoisonError};

    /// Held by each test here: they measure the whole process's memory, so
    /// one running beside another would see its allocations.
    static MEASURING: Mutex<()> = Mutex::new(());

    /// This process's resident anonymous memory, in kB.
    fn resident_anon_kb() -> u64 {
        let status = std::fs::read_to_string("/proc/self/status").expect("the process's status");
        status
            .lines()
            .find_map(|line| line.strip_prefix("RssAnon:"))
            .and_then(|value| value.trim().trim_end_matches("kB").trim().parse().ok())
            .expect("an RssAnon line")
    }

    #[test]
    fn a_large_buffer_freed_once_configured_is_not_kept() {
        let _measuring = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
        super::configure();
        // Freed, a mapped block would raise glibc's thresholds to its size,
        // and the smaller one after it would stay in the heap.
        drop(vec![1_u8; 8 * 1024 * 1024]);
        let before = resident_anon_kb();
        let smaller = vec![1_u8; 4 * 1024 * 1024];
        assert!(smaller[smaller.len() - 1] == 1);
        drop(smaller);
        let after = resident_anon_kb();
        assert!(
            after < before + 1024,
            "{before} kB before, {after} kB after freeing 4096 kB"
        );
    }

    #[test]
    fn the_free_pages_of_a_closed_panel_go_back_to_the_system() {
        let _measuring = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
        // Small blocks, as a panel's layout is made of, so they come from the
        // heap rather than each being mapped and unmapped on its own; the one
        // kept above them stops glibc trimming the heap's top by itself.
        const BLOCKS: usize = 16 * 1024;
        const BLOCK: usize = 1024;
        let mut blocks: Vec<Box<[u8]>> = (0..BLOCKS)
            .map(|index| vec![u8::try_from(index % 251).unwrap_or(0); BLOCK].into_boxed_slice())
            .collect();
        let kept = vec![1_u8; BLOCK].into_boxed_slice();
        blocks.clear();
        blocks.shrink_to_fit();
        let before = resident_anon_kb();
        super::give_back();
        let after = resident_anon_kb();
        assert!(kept[0] == 1);
        let freed = (BLOCKS * BLOCK / 1024) as u64;
        assert!(
            before.saturating_sub(after) > freed / 2,
            "{before} kB before, {after} kB after freeing {freed} kB"
        );
    }
}
