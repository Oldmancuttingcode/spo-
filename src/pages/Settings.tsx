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

type ListeningHistorySyncResult = {
  processedItems: number;
  newPlayHistoryRows: number;
  newTrackRows: number;
  updatedTrackRows: number;
  newArtistRows: number;
  skippedItems: number;
  newDiscoveries: number;
  latestProcessedPlayedAt?: string | null;
  lastSuccessfulSyncAt: string;
};

export default function Settings() {
  const [spotifyStatus, setSpotifyStatus] =
    useState<SpotifyConnectionStatus | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isDisconnecting, setIsDisconnecting] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);
  const [syncResult, setSyncResult] =
    useState<ListeningHistorySyncResult | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);

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
      setSyncResult(null);
      setSyncError(null);
    } catch (error) {
      setSpotifyStatus({
        state: "authentication_error",
        message: String(error),
      });
    } finally {
      setIsDisconnecting(false);
    }
  }

  async function syncRecentlyPlayed() {
    setIsSyncing(true);
    setSyncError(null);
    try {
      const result = await invoke<ListeningHistorySyncResult>(
        "spotify_sync_recently_played",
      );
      setSyncResult(result);
    } catch (error) {
      setSyncError(String(error));
    } finally {
      setIsSyncing(false);
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
          isSyncing={isSyncing}
          syncResult={syncResult}
          syncError={syncError}
          onSyncRecentlyPlayed={syncRecentlyPlayed}
        />
      </section>
    </section>
  );
}

type SpotifyConnectionContentProps = {
  status: SpotifyConnectionStatus | null;
  isConnecting: boolean;
  isDisconnecting: boolean;
  isSyncing: boolean;
  syncResult: ListeningHistorySyncResult | null;
  syncError: string | null;
  onConnect: () => void;
  onDisconnect: () => void;
  onSyncRecentlyPlayed: () => void;
};

function SpotifyConnectionContent({
  status,
  isConnecting,
  isDisconnecting,
  isSyncing,
  syncResult,
  syncError,
  onConnect,
  onDisconnect,
  onSyncRecentlyPlayed,
}: SpotifyConnectionContentProps) {
  if (isConnecting) {
    return <p className="settings-text">Connecting to Spotify...</p>;
  }

  if (status?.state === "connected") {
    return (
      <div className="settings-stack">
        <p className="spotify-status connected">Connected</p>
        {syncResult ? <SyncSummary syncResult={syncResult} /> : null}
        <div className="settings-actions">
          <button
            className="primary-button"
            disabled={isSyncing}
            type="button"
            onClick={onSyncRecentlyPlayed}
          >
            {isSyncing ? "Syncing Spotify..." : syncError ? "Retry" : "Sync Now"}
          </button>
          <button
            className="secondary-button"
            disabled={isDisconnecting || isSyncing}
            type="button"
            onClick={onDisconnect}
          >
            {isDisconnecting ? "Disconnecting..." : "Disconnect"}
          </button>
        </div>
        {syncError ? (
          <div className="settings-stack">
            <p className="settings-error">Spotify sync failed.</p>
            <p className="settings-text">Your local archive is still available.</p>
            <p className="settings-error">{syncError}</p>
          </div>
        ) : null}
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

function SyncSummary({
  syncResult,
}: {
  syncResult: ListeningHistorySyncResult;
}) {
  return (
    <div className="sync-summary">
      <p className="settings-text">
        Last sync {formatDateTime(syncResult.lastSuccessfulSyncAt)}
      </p>
      <p className="settings-text">
        {syncResult.processedItems} plays processed
      </p>
      <p className="settings-text">
        {syncResult.newPlayHistoryRows} new plays
      </p>
      <p className="settings-text">{syncResult.newTrackRows} new tracks</p>
      {syncResult.skippedItems > 0 ? (
        <p className="settings-text">
          {syncResult.skippedItems} unsupported items skipped
        </p>
      ) : null}
    </div>
  );
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}
