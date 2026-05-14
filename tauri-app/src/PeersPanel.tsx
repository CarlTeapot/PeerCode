import { useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RoomState, PeerInfo } from "./useRoomState";
import "./PeersPanel.css";

interface PeersPanelProps {
  roomState: RoomState | null;
  isHost: boolean;
}

export function PeersPanel({ roomState, isHost }: PeersPanelProps) {
  const [open, setOpen] = useState(false);

  const togglePermission = useCallback(async (peer: PeerInfo) => {
    try {
      await invoke("set_peer_permission", {
        targetClientId: peer.client_id,
        canWrite: !peer.can_write,
      });
    } catch (e) {
      console.error("Failed to set peer permission:", e);
    }
  }, []);

  const sortedPeers = useMemo(() => {
    if (!roomState) return [];
    return [...roomState.peers].sort((a, b) => {
      if (a.is_host !== b.is_host) return a.is_host ? -1 : 1;
      return a.client_id.localeCompare(b.client_id);
    });
  }, [roomState]);

  if (!roomState) return null;

  const peerCount = roomState.peers.length;

  return (
    <>
      <button
        className="peers-panel-toggle"
        onClick={() => setOpen((prev) => !prev)}
        title="Peers"
      >
        👥
        {peerCount > 0 && <span className="badge">{peerCount}</span>}
      </button>

      {open && (
        <div className="peers-panel">
          <div className="peers-panel-header">
            <h3>Peers ({peerCount})</h3>
            <button
              className="close-btn"
              onClick={() => setOpen(false)}
              title="Close"
            >
              ✕
            </button>
          </div>

          <div className="peers-list">
            {sortedPeers.map((peer) => (
              <PeerRow
                key={peer.client_id}
                peer={peer}
                isHost={isHost}
                onToggle={togglePermission}
              />
            ))}
          </div>
        </div>
      )}
    </>
  );
}

interface PeerRowProps {
  peer: PeerInfo;
  isHost: boolean;
  onToggle: (peer: PeerInfo) => Promise<void>;
}

function PeerRow({ peer, isHost, onToggle }: PeerRowProps) {
  const initial = (peer.username || peer.client_id).charAt(0).toUpperCase();

  const showToggle = isHost && !peer.is_host;

  return (
    <div className="peer-row">
      <div className={`peer-avatar ${peer.is_host ? "host" : "guest"}`}>
        {initial}
      </div>

      <div className="peer-info">
        <div className="peer-name">
          {peer.username || `Client ${peer.client_id}`}
        </div>
        <div className={`peer-role ${peer.is_host ? "host-role" : ""}`}>
          {peer.is_host ? "Host" : "Guest"}
        </div>
      </div>

      {showToggle ? (
        <div className="perm-control">
          <span
            className={`perm-label ${peer.can_write ? "write" : "readonly"}`}
          >
            {peer.can_write ? "Can Edit" : "Read Only"}
          </span>
          <button
            className={`perm-toggle ${peer.can_write ? "can-write" : "read-only"}`}
            onClick={() => void onToggle(peer)}
            title={
              peer.can_write ? "Revoke write access" : "Grant write access"
            }
          >
            <span className="knob" />
          </button>
        </div>
      ) : (
        <span className={`perm-badge ${peer.can_write ? "write" : "readonly"}`}>
          {peer.can_write ? "Can Edit" : "Read Only"}
        </span>
      )}
    </div>
  );
}
