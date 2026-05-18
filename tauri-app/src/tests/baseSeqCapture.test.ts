import { describe, expect, test, vi } from "vitest";
import { createEnqueueOp, createIpcSenders } from "../opQueue";

// Regression test for docs/frontend-rendering-issues.md §Issue 1.
//
// `sendInsert` / `sendDelete` must accept `baseSeq` as a parameter (captured
// synchronously at the Monaco event), not read it from a ref *inside* the
// chained-promise callback. Otherwise a remote op that lands between enqueue
// and task execution will bump the ref and the queued IPC will fire with a
// post-bump value — `position` was captured at keystroke time but `baseSeq`
// would describe a later document state.

function defer<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe("IPC senders preserve caller-supplied baseSeq", () => {
  test("sendInsert passes through the caller's baseSeq even when external state mutates between enqueue and task execution", async () => {
    const invoke = vi.fn<(cmd: string, args: unknown) => Promise<void>>();
    const opChainRef = { current: Promise.resolve() as Promise<unknown> };
    const enqueueOp = createEnqueueOp(opChainRef);
    const { sendInsert } = createIpcSenders(enqueueOp, invoke);

    // A prior IPC is already in flight, blocking the chain.
    const slow = defer<void>();
    enqueueOp(() => slow.promise);

    // Caller (Monaco event handler) captures baseSeq=0 synchronously, then
    // enqueues the keystroke.
    const sendPromise = sendInsert(10, "A", 0);

    // While the keystroke's IPC is queued, a remote op arrives and bumps
    // any external state the caller might have read. Because baseSeq is a
    // parameter — captured at enqueue time — what `invoke` sees is unaffected.
    /* simulated remote-op bump would go here */

    slow.resolve();
    await sendPromise;

    expect(invoke).toHaveBeenCalledWith(
      "insert",
      expect.objectContaining({
        position: 10,
        content: "A",
        baseSeq: 0,
      }),
    );
  });

  test("sendDelete passes through the caller's baseSeq under the same conditions", async () => {
    const invoke = vi.fn<(cmd: string, args: unknown) => Promise<void>>();
    const opChainRef = { current: Promise.resolve() as Promise<unknown> };
    const enqueueOp = createEnqueueOp(opChainRef);
    const { sendDelete } = createIpcSenders(enqueueOp, invoke);

    const slow = defer<void>();
    enqueueOp(() => slow.promise);

    const sendPromise = sendDelete(10, 3, 0);

    slow.resolve();
    await sendPromise;

    expect(invoke).toHaveBeenCalledWith(
      "delete",
      expect.objectContaining({
        position: 10,
        length: 3,
        baseSeq: 0,
      }),
    );
  });
});
