mod compositor;
mod error;
mod gui;
mod ns_archive;
mod silica;

use compositor::{dev::GpuHandle, BufferDimensions, CompositorPipeline};
use silica::ProcreateFile;
use std::sync::Arc;

pub use egui_winit::winit;

use crate::compositor::CompositorTarget;

#[tokio::main]
async fn main() {
    let current_dir = std::env::current_dir().expect("Unable to get current working directory");
    let config_path = std::path::Path::new(&current_dir).join("Untitled_Artwork.procreate");

    let dev = Arc::new(GpuHandle::new().await.unwrap());
    let pipeline = CompositorPipeline::new(&dev);

    let (file, gpu_textures) = ProcreateFile::open(config_path, &dev).unwrap();

    let mut target = CompositorTarget::new(dev.clone());
    target
        .data
        .flip_vertices(file.flipped.horizontally, file.flipped.vertically);
    target.set_dimensions(file.size.width, file.size.height);

    for _ in 0..file.orientation {
        target.data.rotate_vertices(true);
        target.set_dimensions(target.dim.height, target.dim.width);
    }

    let new_layer_config = file.layers.clone();
    let background = (!file.background_hidden).then_some(file.background_color);

    let layers = gui::App::linearize_silica_layers(&new_layer_config);

    for unresolved_layer in &layers {
        target.render(
            &pipeline,
            background,
            &[unresolved_layer.clone()],
            &gpu_textures,
        );
        if let Some(texture) = target.output.as_ref() {
            let export_path = std::path::Path::new(&current_dir).join(format!(
                "demo_layers/{}.png",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ));

            let copied_texture = texture.texture.clone(&dev);
            let dim = BufferDimensions::from_extent(copied_texture.size);
            let _ = copied_texture.export(&target.dev, dim, export_path).await;
        }
    }
}
