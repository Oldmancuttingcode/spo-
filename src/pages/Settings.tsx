import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type SpotifyConnectionState =
  | "connected"
  | "not_connected"
  | "authentication_error";

type SpotifyConnectionStatus = {
  state: SpotifyConnectionState;
  message?: string | null;
};

export default function Settings() {
  const [spotifyStatus, setSpotifyStatus] =
    useState<SpotifyConnectionStatus | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isDisconnecting, setIsDisconnecting] = useState(false);

  useEffect(() => {
    void loadSpotifyStatus();
  }, []);

  async function loadSpotifyStatus() {
    try {
      const status = await invoke<SpotifyConnectionStatus>(
        "spotify_connection_status",
      );
      setSpotifyStatus(status);
    } catch (error) {
      setSpotifyStatus({
        state: "authentication_error",
        message: String(error),
      });
    }
  }

  async function connectSpotify() {
    setIsConnecting(true);
    try {
      const status = await invoke<SpotifyConnectionStatus>("spotify_connect");
      setSpotifyStatus(status);
    } catch (error) {
      setSpotifyStatus({
        state: "authentication_error",
        message: String(error),
      });
    } finally {
      setIsConnecting(false);
    }
  }

  async function disconnectSpotify() {
    setIsDisconnecting(true);
    try {
      const status = await invoke<SpotifyConnectionStatus>(
        "spotify_disconnect",
      );
      setSpotifyStatus(status);
    } catch (error) {
      setSpotifyStatus({
        state: "authentication_error",
        message: String(error),
      });
    } finally {
      setIsDisconnecting(false);
    }
  }

  return (
    <section className="page">
      <h1>Settings</h1>

      <section className="settings-section">
        <h2>Spotify</h2>
        <SpotifyConnectionContent
          isConnecting={isConnecting}
          isDisconnecting={isDisconnecting}
          status={spotifyStatus}
          onConnect={connectSpotify}
          onDisconnect={disconnectSpotify}
        />
      </section>
    </section>
  );
}

type SpotifyConnectionContentProps = {
  status: SpotifyConnectionStatus | null;
  isConnecting: boolean;
  isDisconnecting: boolean;
  onConnect: () => void;
  onDisconnect: () => void;
};

function SpotifyConnectionContent({
  status,
  isConnecting,
  isDisconnecting,
  onConnect,
  onDisconnect,
}: SpotifyConnectionContentProps) {
  if (isConnecting) {
    return <p className="settings-text">Connecting to Spotify...</p>;
  }

  if (status?.state === "connected") {
    return (
      <div className="settings-stack">
        <p className="spotify-status connected">Connected</p>
        <button
          className="secondary-button"
          disabled={isDisconnecting}
          type="button"
          onClick={onDisconnect}
        >
          {isDisconnecting ? "Disconnecting..." : "Disconnect"}
        </button>
      </div>
    );
  }

  if (status?.state === "authentication_error") {
    return (
      <div className="settings-stack">
        <p className="settings-text">Could not connect to Spotify.</p>
        {status.message ? (
          <p className="settings-error">{status.message}</p>
        ) : null}
        <button className="primary-button" type="button" onClick={onConnect}>
          Try Again
        </button>
      </div>
    );
  }

  return (
    <div className="settings-stack">
      <p className="settings-text">
        Connect Spotify to sync your listening history.
      </p>
      <button className="primary-button" type="button" onClick={onConnect}>
        Connect Spotify
      </button>
    </div>
  );
}
