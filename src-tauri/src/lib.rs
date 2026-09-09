mod database;
pub mod music;

use tauri::Manager;
use tauri::State;

#[tauri::command]
fn database_health(database: State<'_, database::Database>) -> Result<String, String> {
    database.health()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database = database::connect(app.handle())?;
            println!(
                "Music Archive database initialized at {}",
                database.path().display()
            );
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![database_health])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
