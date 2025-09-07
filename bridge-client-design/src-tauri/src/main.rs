// #![cfg_attr(
//   all(not(debug_assertions), target_os = "windows"),
//   windows_subsystem = "windows"
// )]

// fn main() {
//   let context = tauri::generate_context!();
//   tauri::Builder::default()
//     .menu(tauri::Menu::os_default(&context.package_info().name))
//     .run(context)
//     .expect("error while running tauri application");
// }

#[tauri::command]
fn native_list_topics() -> Vec<serde_json::Value> {
vec![serde_json::json!({"name":"/dummy","type":"std_msgs/String"})]
}


fn main() {
tauri::Builder::default()
.invoke_handler(tauri::generate_handler![native_list_topics])
.run(tauri::generate_context!())
.expect("error while running tauri application");
}