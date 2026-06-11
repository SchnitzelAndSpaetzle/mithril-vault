// SPDX-License-Identifier: MIT

use std::path::PathBuf;
use std::sync::Mutex;

/// Holds the file paths of an in-flight attachment-add gesture so the trusted
/// add path can read them without the renderer ever naming a file. Paths are
/// captured *in Rust* from a trusted OS origin (the native `tauri://drag-drop`
/// window event, or the file-picker dialog) and stashed here (see ADR-0004);
/// the renderer never supplies a path — there is no command parameter through
/// which one could be passed.
///
/// The add flow is two-phase so the soft-warning prompt can fit between picking
/// and storing: the prepare step buffers paths here and inspects their sizes via
/// [`peek`](PendingAttachmentPaths::peek) (a non-draining clone), and the commit
/// step drains them with [`take`](PendingAttachmentPaths::take) and feeds them to
/// `KdbxService::add_entry_attachments`. Because the renderer commits
/// synchronously within a single gesture, only one batch is ever buffered at a
/// time; a fresh pick or drop overwrites the previous buffer.
#[derive(Default)]
pub struct PendingAttachmentPaths {
    paths: Mutex<Vec<PathBuf>>,
}

impl PendingAttachmentPaths {
    /// Overwrites the buffer with the paths from a fresh pick or drop.
    pub fn replace(&self, paths: Vec<PathBuf>) {
        if let Ok(mut guard) = self.paths.lock() {
            *guard = paths;
        }
    }

    /// Returns a clone of the buffered paths without draining them, so the
    /// prepare step can classify their sizes while leaving them in place for the
    /// commit step that follows a confirmation.
    pub fn peek(&self) -> Vec<PathBuf> {
        match self.paths.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Removes and returns the buffered paths, leaving the buffer empty so a
    /// second commit (or a commit with no preceding pick/drop) reads nothing.
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
        let buffer = PendingAttachmentPaths::default();
        buffer.replace(vec![
            PathBuf::from("/tmp/a.txt"),
            PathBuf::from("/tmp/b.txt"),
        ]);

        let drained = buffer.take();
        assert_eq!(
            drained,
            vec![PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b.txt")],
            "take returns exactly what the pick/drop captured, in order"
        );

        assert!(
            buffer.take().is_empty(),
            "a second commit with no fresh pick/drop reads nothing"
        );
    }

    #[test]
    fn take_on_an_untouched_buffer_returns_nothing() {
        let buffer = PendingAttachmentPaths::default();
        assert!(
            buffer.take().is_empty(),
            "a commit with no preceding pick/drop is a no-op"
        );
    }

    #[test]
    fn peek_returns_paths_without_draining() {
        // The prepare step peeks to classify sizes; the paths must survive for
        // the commit step that follows a confirmation. Two peeks in a row see
        // the same batch, and a subsequent take still drains it.
        let buffer = PendingAttachmentPaths::default();
        buffer.replace(vec![PathBuf::from("/tmp/a.txt")]);

        assert_eq!(buffer.peek(), vec![PathBuf::from("/tmp/a.txt")]);
        assert_eq!(
            buffer.peek(),
            vec![PathBuf::from("/tmp/a.txt")],
            "peek must not drain — a second peek sees the same batch"
        );
        assert_eq!(
            buffer.take(),
            vec![PathBuf::from("/tmp/a.txt")],
            "take after peek still drains the buffer"
        );
    }
}
