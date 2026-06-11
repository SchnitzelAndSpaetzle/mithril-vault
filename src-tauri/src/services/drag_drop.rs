// SPDX-License-Identifier: MIT

use std::path::PathBuf;
use std::sync::Mutex;

/// Holds the file paths from the most recent native `tauri://drag-drop` window
/// event so the trusted add path can read them without the renderer ever naming
/// a file. Paths are captured *in Rust* from the OS event (see ADR-0004) and
/// stashed here; the `commit_dropped_attachments` command drains them with
/// [`take`](DropPathsBuffer::take) and feeds them to
/// `KdbxService::add_entry_attachments`. The renderer decides *whether* to
/// commit (the drop must land on the selected Entry's panel) but never supplies
/// the paths — there is no command parameter through which one could be passed.
///
/// A drop that the renderer chooses not to commit leaves its paths buffered
/// until the next drop overwrites them or a commit drains them; staleness is
/// bounded to a single drop because the renderer commits synchronously in the
/// same drop handler.
#[derive(Default)]
pub struct DropPathsBuffer {
    paths: Mutex<Vec<PathBuf>>,
}

impl DropPathsBuffer {
    /// Overwrites the buffer with the paths from a fresh drop event.
    pub fn replace(&self, paths: Vec<PathBuf>) {
        if let Ok(mut guard) = self.paths.lock() {
            *guard = paths;
        }
    }

    /// Removes and returns the buffered paths, leaving the buffer empty so a
    /// second commit (or a commit with no preceding drop) reads nothing.
    pub fn take(&self) -> Vec<PathBuf> {
        match self.paths.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_drains_the_buffered_paths_and_empties_it() {
        let buffer = DropPathsBuffer::default();
        buffer.replace(vec![
            PathBuf::from("/tmp/a.txt"),
            PathBuf::from("/tmp/b.txt"),
        ]);

        let drained = buffer.take();
        assert_eq!(
            drained,
            vec![PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b.txt")],
            "take returns exactly what the drop captured, in order"
        );

        assert!(
            buffer.take().is_empty(),
            "a second commit with no fresh drop reads nothing"
        );
    }

    #[test]
    fn take_on_an_untouched_buffer_returns_nothing() {
        let buffer = DropPathsBuffer::default();
        assert!(
            buffer.take().is_empty(),
            "a commit with no preceding drop is a no-op"
        );
    }
}
