import os,sqlite3
from pathlib import Path
p=Path(os.environ['APPDATA'])/'com.local.musicarchive.mvpvalidation'/'music-archive.db'
c=sqlite3.connect(p.as_uri()+'?mode=ro',uri=True)
print('Integrity:',c.execute('PRAGMA integrity_check').fetchone()[0])
print('Foreign key issues:',len(c.execute('PRAGMA foreign_key_check').fetchall()))
print('Migrations:',[r[0] for r in c.execute('SELECT version FROM schema_migrations ORDER BY version')])
for t in ['playlists','playlist_tracks','playlist_track_history','track_notes','digging_sessions']:
 print(t,c.execute('SELECT COUNT(*) FROM '+t).fetchone()[0])
print('September local plays/discoveries:',c.execute("SELECT COUNT(*),COUNT(DISTINCT track_id) FROM play_history WHERE played_at >= '2026-08-31T15:00:00Z' AND played_at < '2026-09-30T15:00:00Z'").fetchone()[0],c.execute("SELECT COUNT(*) FROM (SELECT MIN(played_at) d FROM play_history GROUP BY track_id) WHERE d >= '2026-08-31T15:00:00Z' AND d < '2026-09-30T15:00:00Z'").fetchone()[0])
