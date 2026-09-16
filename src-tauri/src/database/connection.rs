use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use super::migrations;

const DATABASE_FILENAME: &str = "music-archive.db";

pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn health(&self) -> Result<String, String> {
        let connection = self.connection.lock().map_err(|_| {
            format!(
                "Database connection lock failed for {}",
                self.path.display()
            )
        })?;

        let foreign_keys_enabled: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .map_err(|error| {
                format!(
                    "Database health check failed for {}: {error}",
                    self.path.display()
                )
            })?;

        if foreign_keys_enabled != 1 {
            return Err(format!(
                "Database foreign key enforcement is disabled for {}",
                self.path.display()
            ));
        }

        Ok("Database ready".to_string())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut connection = self.connection.lock().map_err(|_| {
            format!(
                "Database connection lock failed for {}",
                self.path.display()
            )
        })?;

        operation(&mut connection)
    }
}

pub fn connect(app: &AppHandle) -> Result<Database, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve Music Archive app data directory: {error}"))?;

    fs::create_dir_all(&app_data_dir).map_err(|error| {
        format!(
            "Failed to create Music Archive app data directory at {}: {error}",
            app_data_dir.display()
        )
    })?;

    let database_path = app_data_dir.join(DATABASE_FILENAME);
    let mut connection = open_connection(&database_path)?;
    migrations::run(&mut connection, &database_path)?;

    Ok(Database {
        connection: Mutex::new(connection),
        path: database_path,
    })
}

fn open_connection(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| {
        format!(
            "Failed to open SQLite database at {}: {error}",
            path.display()
        )
    })?;

    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| {
            format!(
                "Failed to enable SQLite foreign keys for {}: {error}",
                path.display()
            )
        })?;

    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::open_connection;

    #[test]
    fn open_connection_enables_foreign_keys() {
        let connection = open_connection(":memory:".as_ref()).expect("connection opens");

        let enabled: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign key pragma can be read");

        assert_eq!(enabled, 1);
    }

    #[test]
    #[ignore = "reads the local app-data database for manual validation"]
    fn local_app_data_database_counts_phase_five_tables() {
        let database_path =
            std::path::PathBuf::from(std::env::var("APPDATA").expect("APPDATA is set"))
                .join("com.local.musicarchive")
                .join("music-archive.db");
        let connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("local app-data database opens read-only");

        for table in [
            "artists",
            "tracks",
            "track_artists",
            "play_history",
            "sync_state",
        ] {
            let query = format!("SELECT COUNT(*) FROM {table}");
            let count: i64 = connection
                .query_row(&query, [], |row| row.get(0))
                .expect("table count can be read");
            println!("{table}={count}");
        }
    }
}
