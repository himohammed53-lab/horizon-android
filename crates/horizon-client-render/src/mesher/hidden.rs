//! Blocks the mesher leaves out of a section for a moment. The client draws a block that was
//! just placed itself while it pops in (`game/placement_animation.rs`), so the section mesh
//! must not draw it too. Shared by the mesh workers, so it is a process-wide list; it holds a
//! handful of positions at most and is empty when the animation is off.
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

static HIDDEN: Mutex<Vec<[i32; 3]>> = Mutex::new(Vec::new());
/// The list's length, so the mesher's hot loop skips the lock while nothing is hidden.
static COUNT: AtomicUsize = AtomicUsize::new(0);

fn list() -> std::sync::MutexGuard<'static, Vec<[i32; 3]>> {
    HIDDEN.lock().unwrap_or_else(|e| e.into_inner())
}

/// Leaves the block at `pos` out of the meshes built from now on.
pub fn hide(pos: [i32; 3]) {
    let mut hidden = list();
    if !hidden.contains(&pos) {
        hidden.push(pos);
    }
    COUNT.store(hidden.len(), Ordering::Relaxed);
}

/// Puts the block back; true when it was hidden. The caller re-meshes its section.
pub fn show(pos: [i32; 3]) -> bool {
    let mut hidden = list();
    let before = hidden.len();
    hidden.retain(|p| *p != pos);
    COUNT.store(hidden.len(), Ordering::Relaxed);
    hidden.len() != before
}

/// Whether any block is hidden at all (a relaxed read: the mesher's loops ask before the lock).
pub fn any() -> bool {
    COUNT.load(Ordering::Relaxed) != 0
}

/// The hidden positions, for the mesher to prepare a section with.
pub fn positions() -> Vec<[i32; 3]> {
    list().clone()
}

/// Whether the mesher must skip the block at `pos`.
pub fn is_hidden(pos: [i32; 3]) -> bool {
    COUNT.load(Ordering::Relaxed) != 0 && list().contains(&pos)
}

/// Shows every block again (a world closed or changed); returns what was hidden.
pub fn clear() -> Vec<[i32; 3]> {
    let mut hidden = list();
    COUNT.store(0, Ordering::Relaxed);
    std::mem::take(&mut *hidden)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hidden_block_is_skipped_until_it_is_shown() {
        // The list is process-wide, so this test uses positions no other test touches.
        let (a, b) = ([9001, 5, 9001], [9002, 5, 9002]);
        assert!(!is_hidden(a));
        hide(a);
        hide(a);
        hide(b);
        assert!(is_hidden(a) && is_hidden(b));
        assert!(show(a));
        assert!(!show(a));
        assert!(!is_hidden(a) && is_hidden(b));
        assert!(show(b));
        assert!(!is_hidden(b));
    }
}
