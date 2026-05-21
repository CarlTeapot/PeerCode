import {
  useEffect,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";
import { listen } from "@tauri-apps/api/event";

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
}

interface UseSnapshotListenerArgs {
  editorRef: MutableRefObject<editor.IStandaloneCodeEditor | null>;
  monacoRef: MutableRefObject<Monaco | null>;
  isApplyingRemote: MutableRefObject<boolean>;
  eventCountRef: MutableRefObject<number>;
  setEventLog: Dispatch<SetStateAction<LogEntry[]>>;
  shadowTextRef: MutableRefObject<string>;
}

export function useSnapshotListener({
  editorRef,
  monacoRef,
  isApplyingRemote,
  eventCountRef,
  setEventLog,
  shadowTextRef,
}: UseSnapshotListenerArgs) {
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    listen<{ text: string }>("crdt://snapshot-applied", (e) => {
      const ed = editorRef.current;
      const mn = monacoRef.current;
      if (!ed) return;

      const normalizedText = e.payload.text
        .replace(/\r\n/g, "\n")
        .replace(/\r/g, "\n");

      isApplyingRemote.current = true;
      try {
        ed.setValue(normalizedText);
        if (mn) {
          ed.getModel()?.setEOL(mn.editor.EndOfLineSequence.LF);
        }
      } finally {
        shadowTextRef.current = normalizedText;
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
  }, [
    editorRef,
    monacoRef,
    isApplyingRemote,
    eventCountRef,
    setEventLog,
    shadowTextRef,
  ]);
}
