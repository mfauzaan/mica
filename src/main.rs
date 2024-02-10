mod app;
mod compositor;
mod error;
mod ns_archive;
mod silica;

use app::App;
use compositor::dev::GpuHandle;

pub use egui_winit::winit;

#[tokio::main]
async fn main() {
    let current_dir = std::env::current_dir().expect("Unable to get current working directory");
    let config_path = std::path::Path::new(&current_dir).join("Untitled_Artwork.procreate");

    let dev = GpuHandle::new().await.unwrap();
    let app = App::new(dev);

    let (file, gpu_textures, target) = app.load_file(config_path).await.unwrap();

    app.extract_layers_export(&file, &gpu_textures, target, current_dir)
        .await;
}
