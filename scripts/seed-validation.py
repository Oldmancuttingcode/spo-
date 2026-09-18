"""Seed only the explicitly named disposable validation archive."""
import os
import sqlite3
from pathlib import Path

path = Path(os.environ['APPDATA']) / 'com.local.musicarchive.mvpvalidation' / 'music-archive.db'
if not path.exists():
    raise SystemExit('Create the disposable validation archive first.')
with sqlite3.connect(path) as connection:
    connection.execute("INSERT OR IGNORE INTO playlists(spotify_id,name,track_count,snapshot_id,last_synced_at) VALUES('mvpqa','MVP QA Playlist',2,'fixture','2026-09-16T00:00:00Z')")
    playlist_id = connection.execute("SELECT id FROM playlists WHERE spotify_id='mvpqa'").fetchone()[0]
    track_id = connection.execute('SELECT id FROM tracks ORDER BY id LIMIT 1').fetchone()[0]
    connection.executemany('INSERT OR IGNORE INTO playlist_tracks(playlist_id,track_id,position) VALUES(?,?,?)', [(playlist_id,track_id,0),(playlist_id,track_id,1)])
print('Seeded isolated playlist fixture')
