use crate::database::Database;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Tag {
    id: i64,
    name: String,
    category: String,
}

fn database_error(_: rusqlite::Error) -> String {
    "Could not update or load tags. Please try again.".into()
}

fn validate_category(category: &str) -> Result<(), String> {
    match category {
        "genre" | "mood" | "sound" | "vocal" | "free" => Ok(()),
        _ => Err("Choose a valid tag category.".into()),
    }
}

fn query_tags(
    connection: &Connection,
    category: Option<&str>,
    track_id: Option<i64>,
) -> Result<Vec<Tag>, String> {
    if let Some(category) = category {
        validate_category(category)?;
    }
    let mut statement = connection
        .prepare(
            "SELECT id, name, category FROM tags
         WHERE (?1 IS NULL OR category = ?1)
         AND (?2 IS NULL OR id IN (SELECT tag_id FROM track_tags WHERE track_id = ?2))",
        )
        .map_err(database_error)?;
    let mut tags = statement
        .query_map(params![category, track_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
            })
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    tags.sort_by_cached_key(|tag| (tag.name.to_lowercase(), tag.id));
    Ok(tags)
}

fn require_track(connection: &Connection, id: i64) -> Result<(), String> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tracks WHERE id = ?1)",
            [id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if exists {
        Ok(())
    } else {
        Err("This track was not found in your archive.".into())
    }
}

fn create(connection: &mut Connection, name: &str, category: &str) -> Result<Tag, String> {
    validate_category(category)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a tag name.".into());
    }
    // Serialize lookup + insert, including across SQLite connections. Preserve display casing.
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let normalized = name.to_lowercase();
    let existing = query_tags(&transaction, Some(category), None)?
        .into_iter()
        .find(|tag| tag.name.trim().to_lowercase() == normalized);
    let tag = if let Some(tag) = existing {
        tag
    } else {
        transaction
            .execute(
                "INSERT INTO tags (name, category) VALUES (?1, ?2)",
                params![name, category],
            )
            .map_err(database_error)?;
        Tag {
            id: transaction.last_insert_rowid(),
            name: name.into(),
            category: category.into(),
        }
    };
    transaction.commit().map_err(database_error)?;
    Ok(tag)
}

fn change_assignment(
    connection: &mut Connection,
    track_id: i64,
    tag_id: i64,
    assign: bool,
) -> Result<(), String> {
    let transaction = connection.transaction().map_err(database_error)?;
    require_track(&transaction, track_id)?;
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
            [tag_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !exists {
        return Err("This tag was not found. Reopen the tag picker and try again.".into());
    }
    let sql = if assign {
        "INSERT INTO track_tags (track_id, tag_id) VALUES (?1, ?2) ON CONFLICT(track_id, tag_id) DO NOTHING"
    } else {
        "DELETE FROM track_tags WHERE track_id = ?1 AND tag_id = ?2"
    };
    transaction
        .execute(sql, params![track_id, tag_id])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

#[tauri::command]
pub fn track_tags(database: State<'_, Database>, track_id: i64) -> Result<Vec<Tag>, String> {
    database.with_connection(|connection| {
        require_track(connection, track_id)?;
        query_tags(connection, None, Some(track_id))
    })
}

#[tauri::command]
pub fn list_tags(
    database: State<'_, Database>,
    category: Option<String>,
) -> Result<Vec<Tag>, String> {
    database.with_connection(|connection| query_tags(connection, category.as_deref(), None))
}

#[tauri::command]
pub fn create_tag(
    database: State<'_, Database>,
    name: String,
    category: String,
) -> Result<Tag, String> {
    database.with_connection(|connection| create(connection, &name, &category))
}

#[tauri::command]
pub fn assign_tag_to_track(
    database: State<'_, Database>,
    track_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    database.with_connection(|connection| change_assignment(connection, track_id, tag_id, true))
}

#[tauri::command]
pub fn remove_tag_from_track(
    database: State<'_, Database>,
    track_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    database.with_connection(|connection| change_assignment(connection, track_id, tag_id, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        crate::database::migrations::run(&mut connection, ":memory:".as_ref()).unwrap();
        connection
            .execute_batch("INSERT INTO tracks (id, spotify_id, title) VALUES (1, 'first', 'First'), (2, 'second', 'Second')")
            .unwrap();
        connection
    }

    #[test]
    fn names_preserve_casing_and_duplicates_reuse_existing_tag() {
        let mut db = fixture();
        let tag = create(&mut db, "  Alternative R&B  ", "genre").unwrap();
        assert_eq!(tag.name, "Alternative R&B");
        assert_eq!(tag.category, "genre");
        assert_eq!(
            create(&mut db, "alternative r&b", "genre").unwrap().id,
            tag.id
        );
        assert_ne!(
            create(&mut db, "Alternative R&B", "free").unwrap().id,
            tag.id
        );
        let unicode = create(&mut db, "Été", "mood").unwrap();
        assert_eq!(create(&mut db, "été", "mood").unwrap().id, unicode.id);
        assert_eq!(query_tags(&db, None, None).unwrap().len(), 3);
        assert_eq!(
            query_tags(&db, Some("genre"), None).unwrap()[0].name,
            "Alternative R&B"
        );
    }

    #[test]
    fn invalid_inputs_and_database_failure_are_safe() {
        let mut db = fixture();
        assert!(create(&mut db, "Dark", "invalid").is_err());
        assert!(create(&mut db, " \t\n", "mood").is_err());
        assert!(query_tags(&db, Some("invalid"), None).is_err());
        let tag = create(&mut db, "Dark", "mood").unwrap();
        for id in [-1, 0, 999] {
            assert!(require_track(&db, id).is_err());
            assert!(change_assignment(&mut db, id, tag.id, true).is_err());
            assert!(change_assignment(&mut db, 1, id, true).is_err());
            assert!(change_assignment(&mut db, 1, id, false).is_err());
        }
        let mut empty = Connection::open_in_memory().unwrap();
        assert_eq!(
            create(&mut empty, "Dark", "mood").unwrap_err(),
            "Could not update or load tags. Please try again."
        );
    }

    #[test]
    fn assignments_are_idempotent_and_removal_preserves_shared_tag() {
        let mut db = fixture();
        let tag = create(&mut db, "Dark", "mood").unwrap();
        change_assignment(&mut db, 1, tag.id, true).unwrap();
        change_assignment(&mut db, 1, tag.id, true).unwrap();
        change_assignment(&mut db, 2, tag.id, true).unwrap();
        assert_eq!(query_tags(&db, None, Some(1)).unwrap().len(), 1);
        change_assignment(&mut db, 1, tag.id, false).unwrap();
        change_assignment(&mut db, 1, tag.id, false).unwrap();
        assert!(query_tags(&db, None, Some(1)).unwrap().is_empty());
        assert_eq!(query_tags(&db, None, Some(2)).unwrap()[0].id, tag.id);
        assert_eq!(query_tags(&db, None, None).unwrap().len(), 1);
    }

    #[test]
    fn all_categories_and_alphabetical_order() {
        let mut db = fixture();
        for category in ["genre", "mood", "sound", "vocal", "free"] {
            create(&mut db, "warm", category).unwrap();
            create(&mut db, "Dark", category).unwrap();
            let tags = query_tags(&db, Some(category), None).unwrap();
            assert_eq!(
                tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>(),
                ["Dark", "warm"]
            );
        }
    }

    #[test]
    fn tags_and_removal_persist_after_reopening_database() {
        let path = std::env::temp_dir().join(format!(
            "music-archive-tags-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut db = Connection::open(&path).unwrap();
        crate::database::migrations::run(&mut db, &path).unwrap();
        db.execute(
            "INSERT INTO tracks (id, spotify_id, title) VALUES (1, 'track', 'Track')",
            [],
        )
        .unwrap();
        let tag = create(&mut db, "Late Night", "free").unwrap();
        change_assignment(&mut db, 1, tag.id, true).unwrap();
        drop(db);
        let mut db = Connection::open(&path).unwrap();
        assert_eq!(
            query_tags(&db, None, Some(1)).unwrap()[0].name,
            "Late Night"
        );
        change_assignment(&mut db, 1, tag.id, false).unwrap();
        drop(db);
        let db = Connection::open(&path).unwrap();
        assert!(query_tags(&db, None, Some(1)).unwrap().is_empty());
        assert_eq!(query_tags(&db, None, None).unwrap().len(), 1);
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
