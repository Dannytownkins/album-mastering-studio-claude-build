import { describe, expect, it } from "vitest";

import {
  appendToPast,
  applyRedo,
  applyUndo,
  shouldCoalesceCommit,
} from "./history-stack";

// Mechanical gates for the undo/redo stack math the hook wires up.
// Pre-extraction these were buried inside React-callback closures
// with ref-assignment side effects; as pure functions they exercise
// the boundary conditions that matter most: the coalesce window, the
// max-size pruning, and empty-stack no-ops.

describe("shouldCoalesceCommit", () => {
  it("collapses commits within the coalesce window", () => {
    expect(shouldCoalesceCommit(1_000, 1_100, 300)).toBe(true);
    expect(shouldCoalesceCommit(1_000, 1_299, 300)).toBe(true);
  });

  it("does NOT collapse commits AT or beyond the coalesce window edge", () => {
    expect(shouldCoalesceCommit(1_000, 1_300, 300)).toBe(false);
    expect(shouldCoalesceCommit(1_000, 5_000, 300)).toBe(false);
  });

  it("handles a freshly-reset window (lastCommitAt = 0) as not coalescing", () => {
    // The hook sets lastCommitAt = 0 after an undo / redo so the next
    // user edit always starts a new snapshot. Reaching this code path
    // with any `now > coalesceMs` must return false.
    expect(shouldCoalesceCommit(0, 1_000, 300)).toBe(false);
  });
});

describe("appendToPast", () => {
  it("appends below the size cap", () => {
    const past = [1, 2, 3];
    expect(appendToPast(past, 4, 10)).toEqual([1, 2, 3, 4]);
  });

  it("prunes oldest entries when at the size cap", () => {
    // At cap (length 5, max 5), appending should drop the oldest so
    // length stays at 5.
    const past = [1, 2, 3, 4, 5];
    expect(appendToPast(past, 6, 5)).toEqual([2, 3, 4, 5, 6]);
  });

  it("prunes correctly when past is somehow OVER the cap (defensive)", () => {
    // Shouldn't happen in normal flow, but if it did the function
    // should still produce a maxSize-bounded result.
    const past = [1, 2, 3, 4, 5, 6, 7];
    const result = appendToPast(past, 8, 5);
    expect(result.length).toBe(5);
    expect(result[result.length - 1]).toBe(8);
  });

  it("does not mutate the input array", () => {
    const past = [1, 2, 3];
    const result = appendToPast(past, 4, 10);
    expect(past).toEqual([1, 2, 3]);
    expect(result).not.toBe(past);
  });
});

describe("applyUndo", () => {
  it("returns no-op result when past is empty", () => {
    const result = applyUndo([], [], "current");
    expect(result.restored).toBeNull();
    expect(result.past).toEqual([]);
    expect(result.future).toEqual([]);
  });

  it("pops the most recent past entry, pushes current to future, returns popped", () => {
    const result = applyUndo(["a", "b", "c"], ["x"], "current");
    expect(result.restored).toBe("c");
    expect(result.past).toEqual(["a", "b"]);
    expect(result.future).toEqual(["x", "current"]);
  });

  it("does not mutate the input arrays", () => {
    const past = ["a", "b"];
    const future = ["x"];
    applyUndo(past, future, "current");
    expect(past).toEqual(["a", "b"]);
    expect(future).toEqual(["x"]);
  });
});

describe("applyRedo", () => {
  it("returns no-op result when future is empty", () => {
    const result = applyRedo(["a"], [], "current");
    expect(result.restored).toBeNull();
    expect(result.past).toEqual(["a"]);
    expect(result.future).toEqual([]);
  });

  it("pops the most recent future entry, pushes current to past, returns popped", () => {
    const result = applyRedo(["a"], ["x", "y"], "current");
    expect(result.restored).toBe("y");
    expect(result.past).toEqual(["a", "current"]);
    expect(result.future).toEqual(["x"]);
  });

  it("does not mutate the input arrays", () => {
    const past = ["a"];
    const future = ["x", "y"];
    applyRedo(past, future, "current");
    expect(past).toEqual(["a"]);
    expect(future).toEqual(["x", "y"]);
  });
});

describe("undo → redo round trip", () => {
  it("redo after undo restores the originally-popped state", () => {
    // Start at state "v2" with "v1" in history.
    // Undo: past=[], future=["v2"], restored="v1". Current becomes v1.
    // Redo: past=["v1"], future=[], restored="v2". Current becomes v2.
    const past0 = ["v1"];
    const future0: string[] = [];
    const current0 = "v2";

    const undone = applyUndo(past0, future0, current0);
    expect(undone.restored).toBe("v1");
    expect(undone.past).toEqual([]);
    expect(undone.future).toEqual(["v2"]);

    // After the hook "applies restored," current is now "v1".
    const redone = applyRedo(undone.past, undone.future, "v1");
    expect(redone.restored).toBe("v2");
    expect(redone.past).toEqual(["v1"]);
    expect(redone.future).toEqual([]);
  });
});
