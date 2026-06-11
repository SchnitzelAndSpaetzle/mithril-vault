# Attachment Add: Trusted-Source Path Provenance

Adding a file to an Entry reads bytes off the host filesystem. The **provenance rule** for those paths is: a path that names a file to import may originate **only from a trusted OS source** — the native file dialog opened in Rust, or (for #286) the native `tauri://drag-drop` window event. The renderer never supplies a filesystem path for the add, and the backend has no command parameter through which one could be passed. A path the user never selected is therefore not merely rejected — it is structurally impossible to name.

This closes the defense-in-depth gap recorded in ADR-0003: the initial add slice (#283) took a renderer-supplied `source_path` string over IPC and read whatever it named. Combined with the on-demand byte fetch and export, that made add an arbitrary local-file-read primitive — any code running in the renderer (a future XSS sink, a malicious frontend dependency) could import e.g. `~/.ssh/id_rsa` into the Vault and read it back out. There is no known XSS path today (the app renders only trusted bundled content and React auto-escapes), so this was latent, not live; but the preview (#287) and drag-and-drop (#286) slices widen the renderer surface, so the boundary is closed before/with them.

## Decision

- The `add_entry_attachments` command takes only `db_id` and the Entry `id`. It opens the native multi-select dialog **in Rust** via `tauri-plugin-dialog`'s `app.dialog().file().blocking_pick_files()`, converts each returned `FilePath` to a `PathBuf`, and hands the list to `KdbxService::add_entry_attachments`.
- `KdbxService::add_entry_attachments` is the **single feeder** for the per-file read (`KdbxService::add_entry_attachment`, which keeps the size-cap / TOCTOU / auto-rename guards from #283). It is handed only OS-provided paths and never a string from JS. Drag-and-drop (#286) will reuse this same feeder, passing the paths the `tauri://drag-drop` event provides — it must not introduce a second, JS-routed path into the read.
- A cancelled dialog returns no paths and is a no-op (empty outcome). One bad pick (over the cap, non-regular file, …) is collected into the outcome's `failed` list and never aborts the rest.

## Considered Options

- **Acquire the path in Rust (chosen).** Moving the picker into the command eliminates the JS-held path entirely, so the trust boundary is enforced by the command's shape rather than by a runtime check that could be bypassed or misconfigured. Tauri's own dialog guest API documents this trade-off ("When security is more important than the ease of use of this API, prefer writing a dedicated command instead"). The cost is a wider IPC change (the add command and its frontend wrapper change shape, and the multi-file add loop moves from the React component into the service) — judged worth it for a structural guarantee over a procedural one.
- **Keep the JS picker, validate against a scoped fs permission.** Retain the renderer's `open()` and, for each dialog-selected path, grant a scoped `fs` permission and validate the renderer-supplied `source_path` against that scope in Rust before reading. Lower churn, but it keeps a JS-held path string and a runtime validation step, and leans on Tauri's scope plumbing being airtight — the path stays nameable from JS and is only rejected after the fact. Rejected: a check you can forget or misconfigure is weaker than a path that cannot be named.

## Consequences

- The renderer can no longer name a file to import; the only inputs to the add are the db/entry ids. The backend test `add_entry_attachments_reads_only_paths_handed_to_it` proves a file never handed to the add path (an empty/cancelled selection while a sensitive file sits on disk) is not read and leaves the Vault untouched.
- The frontend `addAttachments(dbId, id)` wrapper returns a batch outcome (`added` stored names + per-file `failed` entries). The component raises one toast per failure and persists via `database.save` only when something landed — preserving the #283 UX (multi-select, hard cap, auto-rename, immediate save, per-file error toasts) without a renderer-side path.
- The `dialog:default` capability is still required for the export save dialog and the delete confirmation; only the add `open()` use is removed from the renderer.
- Drag-and-drop (#286) inherits this rule for free by feeding `KdbxService::add_entry_attachments` from the native event; it should coordinate with this mechanism rather than duplicate it.
