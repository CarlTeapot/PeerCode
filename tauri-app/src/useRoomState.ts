import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

export interface PeerInfo {
  client_id: string;
  username: string;
  is_host: boolean;
  can_write: boolean;
}

export interface RoomState {
  peers: PeerInfo[];
}

const ROOM_STATE_EVENT = "session://room-state";

export function useRoomState() {
  const [roomState, setRoomState] = useState<RoomState | null>(null);

  useEffect(() => {
    const unlisten = listen<RoomState>(ROOM_STATE_EVENT, (event) => {
      setRoomState(event.payload);
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const clearRoomState = () => setRoomState(null);

  return { roomState, clearRoomState };
}
