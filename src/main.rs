mod app;
mod compositor;
mod error;
mod ns_archive;
mod procreate;

use app::App;
use compositor::dev::GpuHandle;

#[tokio::main]
async fn main() {
    let current_dir = std::env::current_dir().expect("Unable to get current working directory");
    let config_path = std::path::Path::new(&current_dir).join("demo_files/Reference_Blend_File.procreate");
    // let config_path =
        // std::path::Path::new(&current_dir).join("demo_files/Untitled_Artwork.procreate");

    let dev = GpuHandle::new().await.expect("Unable to create GpuHandle");
    let app = App::new(dev);

    let (file, gpu_textures, target) = app
        .load_file_from_path(config_path)
        .await
        .expect("Unable to load file");

    app.extract_layers_and_export(&file, &gpu_textures, target, current_dir)
        .await;

}
