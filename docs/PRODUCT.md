# Product

Music Archive is a personal desktop application for preserving a user's music listening history and taste over time.

## Goal

Record what music the user discovered, when it was discovered, what the user was digging at the time, and how their music taste changes over time.

## Core Concepts

### Track

A recorded song or piece of music. A track may have Spotify metadata, but Music Archive should treat the local archive as the durable source for the user's own history and notes.

### Play

An observed listening event for a track at a specific time.

### Discovery

The first time a track is observed in the user's play history.

Discovery is not the date a track is first inserted into the database. It is derived from play history: the earliest known play event for that track.

### Library

The user's local archive of tracks, plays, tags, notes, playlists, and related personal metadata.

### Tag

A user-owned label for organizing tracks, playlists, discoveries, or periods of listening.

Tag categories are Genre, Mood, Sound, Vocal, and Free.

### Playlist

A user-curated collection or imported Spotify playlist snapshot that helps describe how music was grouped at a point in time.

### Digging

A period where the user is actively exploring an artist, genre, scene, mood, country, era, playlist cluster, or other musical thread.

Digging is not the same as Genre.
