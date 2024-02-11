use crate::compositor::{dev::GpuHandle, tex::GpuTexture};
use crate::compositor::{BufferDimensions, CompositorTarget};
use crate::compositor::{CompositeLayer, CompositorPipeline};
use crate::procreate::{ProcreateError, ProcreateFile, SilicaHierarchy};
use std::path::PathBuf;
use std::sync::Arc;

pub struct App {
    pub dev: Arc<GpuHandle>,
    pub pipeline: CompositorPipeline,
}

impl App {
    pub fn new(dev: GpuHandle) -> Self {
        App {
            pipeline: CompositorPipeline::new(&dev),
            dev: Arc::new(dev),
        }
    }

    pub async fn load_file(
        &self,
        path: PathBuf,
    ) -> Result<(ProcreateFile, GpuTexture, CompositorTarget), ProcreateError> {
        let (file, gpu_textures) = ProcreateFile::open(path, &self.dev).unwrap();

        let mut target = CompositorTarget::new(self.dev.clone());

        target
            .data
            .flip_vertices(file.flipped.horizontally, file.flipped.vertically);
        target.set_dimensions(file.size.width, file.size.height);

        for _ in 0..file.orientation {
            target.data.rotate_vertices(true);
            target.set_dimensions(target.dim.height, target.dim.width);
        }

        Ok((file, gpu_textures, target))
    }

    pub async fn extract_layers_and_export(
        &self,
        file: &ProcreateFile,
        textures: &GpuTexture,
        mut target: CompositorTarget,
        current_dir: PathBuf,
    ) {
        let new_layer_config = file.layers.clone();
        let background = (!file.background_hidden).then_some(file.background_color);

        let layers = App::linearize_silica_layers(&new_layer_config);

        for unresolved_layer in &layers {
            target.render(
                &self.pipeline,
                background,
                &[unresolved_layer.clone()],
                textures,
            );
            if let Some(texture) = target.output.as_ref() {
                let export_path = std::path::Path::new(&current_dir).join(format!(
                    "tmp/demo_layers/{}.png",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ));

                let copied_texture = texture.texture.clone(&self.dev);
                let dim = BufferDimensions::from_extent(copied_texture.size);
                let _ = copied_texture.export(&target.dev, dim, export_path).await;
            }
        }
    }

    /// Transform tree structure of layers into a linear list of
    /// layers for rendering.
    pub fn linearize_silica_layers(layers: &crate::procreate::SilicaGroup) -> Vec<CompositeLayer> {
        fn inner<'a>(
            layers: &'a crate::procreate::SilicaGroup,
            composite_layers: &mut Vec<CompositeLayer>,
            mask_layer: &mut Option<(u32, &'a crate::procreate::SilicaLayer)>,
        ) {
            for layer in layers.children.iter().rev() {
                match layer {
                    SilicaHierarchy::Group(group) if !group.hidden => {
                        inner(group, composite_layers, mask_layer);
                    }
                    SilicaHierarchy::Layer(layer) if !layer.hidden => {
                        if let Some((_, mask_layer)) = mask_layer {
                            if layer.clipped && mask_layer.hidden {
                                continue;
                            }
                        }

                        if !layer.clipped {
                            *mask_layer = Some((layer.image, layer));
                        }

                        composite_layers.push(CompositeLayer {
                            texture: layer.image,
                            clipped: layer.clipped.then(|| mask_layer.unwrap().0),
                            opacity: layer.opacity,
                            blend: layer.blend,
                        });
                    }
                    _ => continue,
                }
            }
        }

        let mut composite_layers = Vec::new();
        inner(layers, &mut composite_layers, &mut None);
        composite_layers
    }
}
