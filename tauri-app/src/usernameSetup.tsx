import { useState, useEffect, useCallback, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

const MAX_LEN = 32;

function sanitize(raw: string): string {
  return [...raw]
    .filter((c) => c.charCodeAt(0) >= 0x20)
    .join("")
    .trim()
    .slice(0, MAX_LEN);
}

const overlayStyle: React.CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(0,0,0,0.75)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  zIndex: 9999,
};

const cardStyle: React.CSSProperties = {
  background: "#1e1e2e",
  border: "1px solid #444",
  borderRadius: 8,
  padding: "28px 32px",
  width: 360,
  display: "flex",
  flexDirection: "column",
  gap: 16,
  color: "#ccc",
  fontFamily: "monospace",
};

const inputStyle: React.CSSProperties = {
  background: "#12121f",
  border: "1px solid #555",
  borderRadius: 4,
  color: "#eee",
  fontFamily: "monospace",
  fontSize: 14,
  padding: "6px 10px",
  width: "100%",
  boxSizing: "border-box",
};

const btnStyle = (disabled: boolean): React.CSSProperties => ({
  background: disabled ? "#333" : "#4a7fd4",
  border: "none",
  borderRadius: 4,
  color: disabled ? "#666" : "#fff",
  cursor: disabled ? "not-allowed" : "pointer",
  fontFamily: "monospace",
  fontSize: 14,
  padding: "7px 0",
  width: "100%",
});

const errorStyle: React.CSSProperties = {
  color: "#f77",
  fontSize: 12,
  minHeight: 16,
};

/**
 * Returns the persisted username once it is available.
 * Safe to call inside a component that is already rendered under UsernameGate
 * (the gate guarantees a username exists before children mount).
 */
export function useIdentityUsername(): string {
  const [username, setUsername] = useState("");
  useEffect(() => {
    invoke<{ username: string | null }>("get_identity").then((id) => {
      if (id.username) setUsername(id.username);
    });
  }, []);
  return username;
}

interface UsernameGateProps {
  children: React.ReactNode;
}

export function UsernameGate({ children }: UsernameGateProps) {
  const [ready, setReady] = useState(false);
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    invoke<{ username: string | null }>("get_identity")
      .then((id) => {
        if (id.username) setReady(true);
      })
      .finally(() => setChecking(false));
  }, []);

  if (checking) return null;
  if (ready) return <>{children}</>;

  return (
    <>
      <FirstRunModal onDone={() => setReady(true)} />
      {/* Render children underneath but blocked by the overlay */}
      <div style={{ pointerEvents: "none", filter: "blur(4px)" }}>
        {children}
      </div>
    </>
  );
}

interface FirstRunModalProps {
  onDone: () => void;
}

function FirstRunModal({ onDone }: FirstRunModalProps) {
  const [value, setValue] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const clean = sanitize(value);
  const canSubmit = clean.length > 0 && !saving;

  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!canSubmit) return;
      setSaving(true);
      setError("");
      try {
        await invoke("set_username", { username: clean });
        onDone();
      } catch (err) {
        setError(String(err));
        setSaving(false);
      }
    },
    [clean, canSubmit, onDone],
  );

  return (
    <div style={overlayStyle}>
      <form style={cardStyle} onSubmit={handleSubmit}>
        <div style={{ fontSize: 16, fontWeight: "bold", color: "#eee" }}>
          Welcome to PeerCode
        </div>
        <div style={{ fontSize: 13, color: "#aaa" }}>
          Choose a display name. Others in your session will see it.
        </div>
        <input
          style={inputStyle}
          autoFocus
          placeholder="Your name"
          maxLength={MAX_LEN}
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
            setError("");
          }}
        />
        <div style={errorStyle}>{error}</div>
        <button
          type="submit"
          style={btnStyle(!canSubmit)}
          disabled={!canSubmit}
        >
          {saving ? "saving…" : "Continue"}
        </button>
      </form>
    </div>
  );
}
