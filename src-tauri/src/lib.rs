use epf_core::{diagnostics::Diagnostics, OpenRequest, Profile, Rule, Snapshot, Store};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> epf_core::Result<T> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_snapshot() -> Result<Snapshot, String> {
    blocking(|| epf_core::service::snapshot(&Store::default_path()?)).await
}

#[tauri::command]
async fn open_ports(request: OpenRequest, confirmed: bool) -> Result<Vec<Rule>, String> {
    blocking(move || epf_core::service::open(&Store::default_path()?, request, confirmed)).await
}

#[tauri::command]
async fn close_rule(id: String, confirmed: bool) -> Result<(), String> {
    blocking(move || epf_core::service::close(&Store::default_path()?, &id, confirmed)).await
}

#[tauri::command]
async fn save_profile(request: OpenRequest) -> Result<Profile, String> {
    blocking(move || epf_core::engine::save_profile(&Store::open(Store::default_path()?)?, request)).await
}

#[tauri::command]
async fn delete_profile(id: String) -> Result<(), String> {
    blocking(move || epf_core::engine::delete_profile(&Store::open(Store::default_path()?)?, &id)).await
}

#[tauri::command]
async fn diagnose(include_public_ip: bool) -> Result<Diagnostics, String> {
    blocking(move || {
        let (network, _) = epf_core::UpnpRouter::discover();
        Ok(epf_core::diagnostics::run(network, include_public_ip))
    }).await
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_snapshot, open_ports, close_rule, save_profile, delete_profile, diagnose])
        .run(tauri::generate_context!())
        .expect("EPF could not start the desktop runtime");
}
