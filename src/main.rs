mod app;
mod compositor;
mod error;
mod ns_archive;
mod procreate;

use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fs::File;
use std::io::Cursor;
use std::io::Write;

use app::App;
use compositor::dev::GpuHandle;
use image::ImageOutputFormat;
use zip::{write::FileOptions, write::ZipWriter};

#[tokio::main]
async fn main() {
    let current_dir = std::env::current_dir().expect("Unable to get current working directory");
    let config_path =
        std::path::Path::new(&current_dir).join("demo_files/Reference_Blend_File.procreate");
    // let config_path =
    //     std::path::Path::new(&current_dir).join("demo_files/Untitled_Artwork.procreate");

    let dev = GpuHandle::new().await.expect("Unable to create GpuHandle");
    let app = App::new(dev);

    let (file, gpu_textures, target) = app
        .load_file_from_path(config_path)
        .await
        .expect("Unable to load file");

    let path = std::path::Path::new("example.zip");
    let custom_file = File::create(&path).expect("Unable to create file");

    let mut zip = ZipWriter::new(custom_file);

    let image_buffers = app
        .extract_image_buffers(&file, &gpu_textures, target)
        .await;

    let image_buffers: Vec<Vec<u8>> = image_buffers
        .into_par_iter()
        .map(|image_buffer| {
            let mut buf = Cursor::new(Vec::new());

            image_buffer
                .write_to(&mut buf, ImageOutputFormat::Png)
                .unwrap();

            let inner_vec = buf.into_inner();

            inner_vec
        })
        .collect();

    for (index, image_buffer) in image_buffers.iter().enumerate() {
        let file_path = format!("image_{}.png", index);

        zip.start_file(file_path, FileOptions::default()).unwrap();

        zip.write_all(&image_buffer[..])
            .expect("Unable to write to zip");
    }

    zip.finish().expect("Unable to finish zip");
    println!("Zip file created successfully!");
}
