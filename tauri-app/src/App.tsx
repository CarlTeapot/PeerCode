import { useState, useRef, useEffect } from "react";
import type { editor } from "monaco-editor";
import Editor, { type OnMount } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface LogEntry {
  id: number;
  html: string;
}

function App() {
  const isDevFeaturesEnabled = import.meta.env.VITE_DEV_FEATURES === "true";
  const [status, setStatus] = useState("loading...");
  const [statusReady, setStatusReady] = useState(false);
  const [eventLog, setEventLog] = useState<LogEntry[]>([]);
  const eventCountRef = useRef(0);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [eventLog]);

  const sendInsert = async (position: number, content: string) => {
    await invoke("insert", { position, content });
  };

  const sendDelete = async (position: number, length: number) => {
    await invoke("delete", { position, length });
  };

  const [loggingEnabled, setLoggingEnabled] = useState(false);
  const toggleLogging = async () => {
    if (!isDevFeaturesEnabled) return;
    await invoke("toggle_crdt_logging");
    setLoggingEnabled((prev) => !prev);
  };

  const handleEditorMount: OnMount = (editor) => {
    setStatus("editor ready");
    setStatusReady(true);

    editor.onDidChangeModelContent(
      (event: editor.IModelContentChangedEvent) => {
        void (async () => {
          for (const change of event.changes) {
            const offset = change.rangeOffset;
            const deleteLen = change.rangeLength;
            const insertText = change.text;

            let opType: string, opClass: string, payload: string;
            if (deleteLen > 0 && insertText.length > 0) {
              opType = "replace";
              opClass = "op-replace";
              payload = `offset=${offset}  deleteLength=${deleteLen}  text=${JSON.stringify(insertText)}`;
            } else if (deleteLen > 0) {
              opType = "delete";
              opClass = "op-delete";
              payload = `offset=${offset}  deleteLength=${deleteLen}`;
            } else {
              opType = "insert";
              opClass = "op-insert";
              payload = `offset=${offset}  text=${JSON.stringify(insertText)}`;
            }

            const wireMessage = JSON.stringify({
              type: opType,
              offset,
              ...(deleteLen > 0 && { length: deleteLen }),
              ...(insertText.length > 0 && { text: insertText }),
            });

            try {
              if (deleteLen > 0) {
                await sendDelete(offset, deleteLen);
              }
              if (insertText.length > 0) {
                await sendInsert(offset, insertText);
              }
            } catch (error) {
              const count = ++eventCountRef.current;
              const errorHtml =
                `<span class="label">#${count}</span>` +
                `<span class="op-delete">[ipc-error]</span> ${String(error)}`;
              setEventLog((prev) => [...prev, { id: count, html: errorHtml }]);
              setStatus("ipc error");
              return;
            }

            const count = ++eventCountRef.current;
            const html =
              `<span class="label">#${count}</span>` +
              `<span class="${opClass}">[${opType}]</span> ${payload}` +
              `<span style="color:#555; margin-left:12px;">→ wire: ${wireMessage}</span>`;

            setEventLog((prev) => [...prev, { id: count, html }]);
          }
        })();
      },
    );
  };

  return (
    <>
      <div className="toolbar">
        <span>Monaco Test Harness</span>
        {isDevFeaturesEnabled && (
          <button
            onClick={toggleLogging}
            style={{
              background: loggingEnabled ? "#4a9" : "#555",
              border: "none",
              color: "white",
              padding: "2px 10px",
              cursor: "pointer",
              borderRadius: "3px",
            }}
          >
            CRDT log {loggingEnabled ? "ON" : "OFF"}
          </button>
        )}
        <span className={`status ${statusReady ? "ready" : ""}`}>{status}</span>
      </div>
      <div className="editor-container">
        <Editor
          height="100%"
          defaultLanguage="rust"
          defaultValue={[].join("\n")}
          theme="vs-dark"
          onMount={handleEditorMount}
          options={{
            fontSize: 14,
            automaticLayout: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
          }}
        />
      </div>
      <div className="log-header">
        change event log ? this is what your rust process will receive
      </div>
      <div className="event-log" ref={logRef}>
        {eventLog.map((entry) => (
          <div
            className="entry"
            key={entry.id}
            dangerouslySetInnerHTML={{ __html: entry.html }}
          />
        ))}
      </div>
    </>
  );
}

export default App;
