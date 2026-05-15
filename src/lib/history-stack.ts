// Pure undo/redo stack math, extracted from the inline implementation
// in `useTrackMaster.ts`. The hook holds React refs + state setters
// and wires up live-audio side effects; the stack arithmetic itself
// lives here so it can be Vitest-gated.
//
// Generic over `T` so any snapshot shape works — the hook plugs in
// its `HistorySnapshot` type (settingsMap + albumIntent +
// overrideAlbum) at the call site.

/// True when a new commit happening at `now` ms should COLLAPSE into
/// the most-recent commit instead of creating a new snapshot. Used by
/// `commitToHistory` to make a slider drag count as one undo step
/// rather than N. The collapse window is the configurable
/// `coalesceMs`.
export function shouldCoalesceCommit(
  lastCommitAt: number,
  now: number,
  coalesceMs: number,
): boolean {
  return now - lastCommitAt < coalesceMs;
}

/// Apply a commit to the past stack. Returns the new past array with
/// the snapshot appended, oldest entries pruned if the array would
/// exceed `maxSize`. Caller is responsible for clearing the future
/// stack (standard undo/redo semantics: a new commit invalidates
/// redo history) — that's a separate concern handled by the caller
/// alongside resetting `lastCommitAt`.
///
/// Splitting the pruning math out of the React ref-assignment makes
/// the "100-step max" boundary testable without spinning up the hook.
export function appendToPast<T>(
  past: readonly T[],
  snapshot: T,
  maxSize: number,
): T[] {
  if (past.length >= maxSize) {
    // Drop the oldest entries to make room; keep at most maxSize total
    // after the append.
    return [...past.slice(past.length - maxSize + 1), snapshot];
  }
  return [...past, snapshot];
}

/// Result of an undo / redo: the new past + future stacks, plus the
/// popped snapshot (if any) for the caller to apply to live state.
/// `restored === null` means the requested operation was a no-op
/// (empty stack); the caller should not touch state in that case.
export interface UndoRedoResult<T> {
  past: T[];
  future: T[];
  restored: T | null;
}

/// Pop the most recent snapshot off the past stack, push the
/// current snapshot onto the future stack. Returns the new stacks
/// and the popped snapshot to restore. No-op when past is empty.
///
/// Standard undo semantics: current state becomes available for
/// redo; one step back becomes the new current.
export function applyUndo<T>(
  past: readonly T[],
  future: readonly T[],
  current: T,
): UndoRedoResult<T> {
  if (past.length === 0) {
    return { past: [...past], future: [...future], restored: null };
  }
  const restored = past[past.length - 1];
  return {
    past: past.slice(0, -1),
    future: [...future, current],
    restored,
  };
}

/// Pop the most recent snapshot off the future stack, push the
/// current snapshot onto the past stack. Returns the new stacks
/// and the popped snapshot to restore. No-op when future is empty.
///
/// Mirror of `applyUndo` for the redo direction.
export function applyRedo<T>(
  past: readonly T[],
  future: readonly T[],
  current: T,
): UndoRedoResult<T> {
  if (future.length === 0) {
    return { past: [...past], future: [...future], restored: null };
  }
  const restored = future[future.length - 1];
  return {
    past: [...past, current],
    future: future.slice(0, -1),
    restored,
  };
}
