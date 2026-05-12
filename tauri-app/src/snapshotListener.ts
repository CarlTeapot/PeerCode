import {
  useEffect,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import type { editor } from "monaco-editor";
import { listen } from "@tauri-apps/api/event";

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
}

interface UseSnapshotListenerArgs {
  editorRef: MutableRefObject<editor.IStandaloneCodeEditor | null>;
  isApplyingRemote: MutableRefObject<boolean>;
  eventCountRef: MutableRefObject<number>;
  setEventLog: Dispatch<SetStateAction<LogEntry[]>>;
}

export function useSnapshotListener({
  editorRef,
  isApplyingRemote,
  eventCountRef,
  setEventLog,
}: UseSnapshotListenerArgs) {
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    listen<{ text: string }>("crdt://snapshot-applied", (e) => {
      const ed = editorRef.current;
      if (!ed) return;

      isApplyingRemote.current = true;
      try {
        ed.setValue(e.payload.text);
      } finally {
        isApplyingRemote.current = false;
      }

      const count = ++eventCountRef.current;
      setEventLog((prev) => [
        ...prev,
        {
          id: count,
          operationClass: "op-snapshot",
          operationLabel: "[snapshot-applied]",
          payload: `text_len=${e.payload.text.length}`,
        },
      ]);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [editorRef, isApplyingRemote, eventCountRef, setEventLog]);
}
