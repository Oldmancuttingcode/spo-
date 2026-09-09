use std::path::Path;

use rusqlite::{params, Connection};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "database foundation",
    sql: "",
}];

pub fn run(connection: &mut Connection, database_path: &Path) -> Result<(), String> {
    ensure_migration_table(connection, database_path)?;

    for migration in MIGRATIONS {
        if has_migration(connection, migration.version, database_path)? {
            continue;
        }

        let transaction = connection.transaction().map_err(|error| {
            format!(
                "Failed to start migration {} for {}: {error}",
                migration.version,
                database_path.display()
            )
        })?;

        if !migration.sql.trim().is_empty() {
            transaction.execute_batch(migration.sql).map_err(|error| {
                format!(
                    "Failed to apply migration {} ({}) for {}: {error}",
                    migration.version,
                    migration.name,
                    database_path.display()
                )
            })?;
        }

        transaction
            .execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                params![migration.version],
            )
            .map_err(|error| {
                format!(
                    "Failed to record migration {} for {}: {error}",
                    migration.version,
                    database_path.display()
                )
            })?;

        transaction.commit().map_err(|error| {
            format!(
                "Failed to commit migration {} for {}: {error}",
                migration.version,
                database_path.display()
            )
        })?;
    }

    Ok(())
}

fn ensure_migration_table(connection: &Connection, database_path: &Path) -> Result<(), String> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )
        .map_err(|error| {
            format!(
                "Failed to ensure schema_migrations table for {}: {error}",
                database_path.display()
            )
        })
}

fn has_migration(
    connection: &Connection,
    version: i64,
    database_path: &Path,
) -> Result<bool, String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![version],
            |row| row.get(0),
        )
        .map_err(|error| {
            format!(
                "Failed to inspect migration {} for {}: {error}",
                version,
                database_path.display()
            )
        })?;

    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::Connection;

    use super::run;

    #[test]
    fn migrations_are_recorded_once() {
        let database_path = std::env::temp_dir().join(format!(
            "music-archive-migration-test-{}.db",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is valid")
                .as_nanos()
        ));

        let mut connection = Connection::open(&database_path).expect("test database opens");

        run(&mut connection, &database_path).expect("first migration run succeeds");
        run(&mut connection, &database_path).expect("second migration run succeeds");

        let migration_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count can be read");

        assert_eq!(migration_count, 1);

        drop(connection);
        fs::remove_file(database_path).expect("test database can be removed");
    }
}
