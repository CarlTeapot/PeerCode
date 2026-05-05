import {
  useEffect,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import type { editor } from "monaco-editor";
import type { Monaco } from "@monaco-editor/react";
import { listen } from "@tauri-apps/api/event";

type RemoteChangeEvent =
  | { type: "insert"; position: number; content: string }
  | { type: "delete"; position: number; length: number };

interface LogEntry {
  id: number;
  operationClass: string;
  operationLabel: string;
  payload: string;
}

interface UseRemoteChangeListenerArgs {
  editorRef: MutableRefObject<editor.IStandaloneCodeEditor | null>;
  monacoRef: MutableRefObject<Monaco | null>;
  isApplyingRemote: MutableRefObject<boolean>;
  eventCountRef: MutableRefObject<number>;
  setEventLog: Dispatch<SetStateAction<LogEntry[]>>;
}

export function useRemoteChangeListener({
  editorRef,
  monacoRef,
  isApplyingRemote,
  eventCountRef,
  setEventLog,
}: UseRemoteChangeListenerArgs) {
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    listen<RemoteChangeEvent>("crdt://remote-change", (e) => {
      const ed = editorRef.current;
      const mn = monacoRef.current;
      if (!ed || !mn) return;

      const model = ed.getModel();
      if (!model) return;

      const change = e.payload;
      isApplyingRemote.current = true;
      try {
        if (change.type === "insert") {
          const pos = model.getPositionAt(change.position);
          ed.executeEdits("remote", [
            {
              range: new mn.Range(
                pos.lineNumber,
                pos.column,
                pos.lineNumber,
                pos.column,
              ),
              text: change.content,
              forceMoveMarkers: true,
            },
          ]);

          const count = ++eventCountRef.current;
          setEventLog((prev) => [
            ...prev,
            {
              id: count,
              operationClass: "op-insert",
              operationLabel: "[remote-insert]",
              payload: `offset=${change.position}  text=${JSON.stringify(change.content)}`,
            },
          ]);
        } else {
          const startPos = model.getPositionAt(change.position);
          const endPos = model.getPositionAt(change.position + change.length);
          ed.executeEdits("remote", [
            {
              range: new mn.Range(
                startPos.lineNumber,
                startPos.column,
                endPos.lineNumber,
                endPos.column,
              ),
              text: "",
              forceMoveMarkers: true,
            },
          ]);

          const count = ++eventCountRef.current;
          setEventLog((prev) => [
            ...prev,
            {
              id: count,
              operationClass: "op-delete",
              operationLabel: "[remote-delete]",
              payload: `offset=${change.position}  length=${change.length}`,
            },
          ]);
        }
      } finally {
        isApplyingRemote.current = false;
      }
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
  }, [editorRef, monacoRef, isApplyingRemote, eventCountRef, setEventLog]);
}
