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
/// step drains them with [`take`](PendingAttachmentPaths::take).
///
/// # Batch identity
///
/// Every buffered batch carries a monotonic **generation**, returned from
/// [`replace`](PendingAttachmentPaths::replace) and captured by the prepare step
/// as a batch id. [`take`](PendingAttachmentPaths::take) drains the batch *only*
/// when the caller's batch id still matches the current generation. This closes
/// a window-global race: the `tauri://drag-drop` handler can `replace` the buffer
/// at any time, so after the user has picked an over-soft file and is staring at
/// the confirmation prompt, an unrelated drop anywhere in the window would
/// otherwise replace the pending paths and the confirmed commit would attach the
/// dropped file instead. Tying commit to the prepared generation makes such a
/// superseded commit a no-op rather than a wrong-file attach.
#[derive(Default)]
pub struct PendingAttachmentPaths {
    inner: Mutex<PendingState>,
}

#[derive(Default)]
struct PendingState {
    /// Monotonic id of the currently buffered batch. Starts at 0 (pristine, no
    /// real batch); every `replace` bumps it, so a legitimate batch id is >= 1.
    generation: u64,
    paths: Vec<PathBuf>,
}

impl PendingAttachmentPaths {
    /// Overwrites the buffer with a fresh batch, bumping the generation and
    /// returning the new id. Any prepared-but-not-yet-committed batch is thereby
    /// superseded: its captured id no longer matches, so its commit drains
    /// nothing.
    pub fn replace(&self, paths: Vec<PathBuf>) -> u64 {
        match self.inner.lock() {
            Ok(mut guard) => {
                guard.generation = guard.generation.wrapping_add(1);
                guard.paths = paths;
                guard.generation
            }
            Err(_) => 0,
        }
    }

    /// Returns the current batch id and a clone of its paths without draining,
    /// so the prepare step can classify their sizes while leaving the batch in
    /// place for the commit that follows a confirmation.
    pub fn peek(&self) -> (u64, Vec<PathBuf>) {
        match self.inner.lock() {
            Ok(guard) => (guard.generation, guard.paths.clone()),
            Err(_) => (0, Vec::new()),
        }
    }

    /// Drains and returns the buffered paths only when `batch_id` still matches
    /// the current generation — i.e. no later pick/drop has superseded the
    /// batch. A superseded (or absent) batch yields nothing, so a confirmed
    /// commit can never attach a file from a different, later gesture, and a
    /// second commit of the same batch reads nothing.
    pub fn take(&self, batch_id: u64) -> Vec<PathBuf> {
        match self.inner.lock() {
            Ok(mut guard) if guard.generation == batch_id => std::mem::take(&mut guard.paths),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_drains_the_buffered_paths_for_a_matching_batch_id() {
        let buffer = PendingAttachmentPaths::default();
        let batch = buffer.replace(vec![
            PathBuf::from("/tmp/a.txt"),
            PathBuf::from("/tmp/b.txt"),
        ]);

        let drained = buffer.take(batch);
        assert_eq!(
            drained,
            vec![PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b.txt")],
            "take returns exactly what the pick/drop captured, in order"
        );

        assert!(
            buffer.take(batch).is_empty(),
            "a second commit of the same batch reads nothing"
        );
    }

    #[test]
    fn take_on_an_untouched_buffer_returns_nothing() {
        let buffer = PendingAttachmentPaths::default();
        assert!(
            buffer.take(0).is_empty(),
            "a commit with no preceding pick/drop is a no-op"
        );
    }

    #[test]
    fn peek_returns_the_batch_id_and_paths_without_draining() {
        let buffer = PendingAttachmentPaths::default();
        let batch = buffer.replace(vec![PathBuf::from("/tmp/a.txt")]);

        let (peeked_id, peeked) = buffer.peek();
        assert_eq!(peeked_id, batch, "peek reports the current batch id");
        assert_eq!(peeked, vec![PathBuf::from("/tmp/a.txt")]);
        assert_eq!(
            buffer.peek().1,
            vec![PathBuf::from("/tmp/a.txt")],
            "peek must not drain — a second peek sees the same batch"
        );
        assert_eq!(
            buffer.take(batch),
            vec![PathBuf::from("/tmp/a.txt")],
            "take after peek still drains the batch"
        );
    }

    #[test]
    fn a_superseding_replace_makes_the_earlier_commit_a_noop() {
        // The race the batch id closes: a first gesture prepares (capturing its
        // id), then a window-global drop replaces the buffer before the first
        // gesture commits. The first commit must drain nothing rather than
        // attach the second gesture's files.
        let buffer = PendingAttachmentPaths::default();
        let first = buffer.replace(vec![PathBuf::from("/tmp/picked.txt")]);
        let second = buffer.replace(vec![PathBuf::from("/tmp/dropped.txt")]);
        assert_ne!(first, second, "each replace mints a fresh batch id");

        assert!(
            buffer.take(first).is_empty(),
            "the superseded batch's commit must attach nothing"
        );
        assert_eq!(
            buffer.take(second),
            vec![PathBuf::from("/tmp/dropped.txt")],
            "the current batch still commits normally"
        );
    }
}
