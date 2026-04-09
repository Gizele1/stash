use tauri::Manager;

#[tauri::command]
pub fn open_graph_window(app: tauri::AppHandle, task_id: String) -> Result<(), String> {
    // If window already exists, focus it and return
    if let Some(window) = app.get_webview_window("graph") {
        window.set_focus().map_err(|e: tauri::Error| e.to_string())?;
        return Ok(());
    }
    // Create new graph window
    tauri::WebviewWindowBuilder::new(
        &app,
        "graph",
        tauri::WebviewUrl::App(format!("graph.html?task_id={}", task_id).into()),
    )
    .title("Intent Graph")
    .inner_size(700.0, 500.0)
    .resizable(true)
    .decorations(true)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}
