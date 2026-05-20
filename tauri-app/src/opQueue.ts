import { invoke } from "@tauri-apps/api/core";
import type { RefObject } from "react";

export type EnqueueOp = <T>(task: () => Promise<T>) => Promise<T>;

type InvokeFn = (
  cmd: string,
  args: Record<string, unknown>,
) => Promise<unknown>;

export function createEnqueueOp(
  opChainRef: RefObject<Promise<unknown>>,
): EnqueueOp {
  return <T>(task: () => Promise<T>): Promise<T> => {
    const next = opChainRef.current.then(task, task);
    opChainRef.current = next.catch(() => undefined);
    return next;
  };
}

export type PendingDelta =
  | { kind: "insert"; at: number; len: number }
  | { kind: "delete"; at: number; len: number };

export interface PendingOp {
  localSeq: number;
  delta: PendingDelta;
}

export interface PendingOpStore {
  /** Mint a fresh localSeq, record a single delta, return the localSeq. */
  push(delta: PendingDelta): number;
  /**
   * Record a replace as two deltas (delete then insert) sharing one localSeq.
   * Returns the shared localSeq.
   */
  pushReplace(at: number, deleteLen: number, insertLen: number): number;
  /** Drop all entries whose localSeq <= seq. */
  pruneAtMost(seq: number): void;
  /** Shift `pos` through every surviving delta in insertion order. */
  transform(pos: number): number;
  /**
   * Clear pending entries without resetting the counter.
   * Deliberately does NOT reset the counter so that localSeq values remain
   * monotonically increasing across snapshot resets, avoiding false pruning
   * of new entries by in-flight remote events that carry stale watermarks.
   */
  reset(): void;
}

export function createPendingOpStore(): PendingOpStore {
  let counter = 0;
  let ops: PendingOp[] = [];

  function shiftOne(delta: PendingDelta, pos: number): number {
    if (delta.kind === "insert") {
      return pos >= delta.at ? pos + delta.len : pos;
    } else {
      if (pos <= delta.at) return pos;
      if (pos >= delta.at + delta.len) return pos - delta.len;
      return delta.at;
    }
  }

  return {
    push(delta) {
      counter += 1;
      ops.push({ localSeq: counter, delta });
      return counter;
    },

    pushReplace(at, deleteLen, insertLen) {
      counter += 1;
      const seq = counter;
      ops.push({
        localSeq: seq,
        delta: { kind: "delete", at, len: deleteLen },
      });
      ops.push({
        localSeq: seq,
        delta: { kind: "insert", at, len: insertLen },
      });
      return seq;
    },

    pruneAtMost(seq) {
      ops = ops.filter((e) => e.localSeq > seq);
    },

    transform(pos) {
      let p = pos;
      for (const { delta } of ops) {
        p = shiftOne(delta, p);
      }
      return p;
    },

    reset() {
      ops = [];
    },
  };
}

export interface IpcSenders {
  sendInsert: (
    position: number,
    content: string,
    baseSeq: number,
  ) => Promise<unknown>;
  sendDelete: (
    position: number,
    length: number,
    baseSeq: number,
  ) => Promise<unknown>;
  sendReplace: (
    position: number,
    deleteLength: number,
    content: string,
    baseSeq: number,
  ) => Promise<unknown>;
}

export function createIpcSenders(
  enqueueOp: EnqueueOp,
  store: PendingOpStore,
  invokeFn: InvokeFn = invoke,
): IpcSenders {
  return {
    sendInsert(position, content, baseSeq) {
      const localSeq = store.push({
        kind: "insert",
        at: position,
        len: [...content].length,
      });
      return enqueueOp(() =>
        invokeFn("insert", { position, content, baseSeq, localSeq }),
      );
    },

    sendDelete(position, length, baseSeq) {
      const localSeq = store.push({
        kind: "delete",
        at: position,
        len: length,
      });
      return enqueueOp(() =>
        invokeFn("delete", { position, length, baseSeq, localSeq }),
      );
    },

    sendReplace(position, deleteLength, content, baseSeq) {
      const localSeq = store.pushReplace(
        position,
        deleteLength,
        [...content].length,
      );
      return enqueueOp(() =>
        invokeFn("replace", {
          position,
          deleteLength,
          content,
          baseSeq,
          localSeq,
        }),
      );
    },
  };
}
