//! Giving the heap's free pages back when a panel closes
//! (`docs/plan/12-performance-and-power.md`).

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
        // SAFETY: `malloc_trim` takes any padding and only releases memory
        // the allocator holds free, so no argument can make it unsound.
        unsafe extern "C" {
            safe fn malloc_trim(pad: usize) -> std::ffi::c_int;
        }
        // It answers whether any memory was released, which changes nothing.
        let _released = malloc_trim(0);
    }
}

#[cfg(all(test, target_os = "linux", target_env = "gnu"))]
mod tests {
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
    fn the_free_pages_of_a_closed_panel_go_back_to_the_system() {
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
