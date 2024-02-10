use crate::compositor::CompositorTarget;
use crate::compositor::{dev::GpuHandle, tex::GpuTexture};
use crate::compositor::{CompositeLayer, CompositorPipeline};
use crate::silica::{ProcreateFile, SilicaError, SilicaHierarchy};
use egui_dock::{NodeIndex, SurfaceIndex};
use egui_notify::Toasts;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct App {
    pub dev: Arc<GpuHandle>,
    pub rt: Arc<Runtime>,
    pub compositor: CompositorHandle,
    pub toasts: Mutex<Toasts>,
    pub added_instances: Mutex<Vec<(SurfaceIndex, NodeIndex, InstanceKey)>>,
}


#[derive(Hash, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct InstanceKey(pub usize);

pub struct Instance {
    pub file: RwLock<ProcreateFile>,
    pub textures: GpuTexture,
    pub target: Mutex<CompositorTarget>,
    pub changed: AtomicBool,
}

impl Drop for Instance {
    fn drop(&mut self) {
        println!("Closing {:?}", self.file.get_mut().name);
    }
}

pub struct CompositorHandle {
    pub instances: RwLock<HashMap<InstanceKey, Instance>>,
    pub curr_id: AtomicUsize,
    pub pipeline: CompositorPipeline,
}

impl App {
    pub fn new(dev: GpuHandle, rt: Arc<Runtime>) -> Self {
        App {
            compositor: CompositorHandle {
                instances: RwLock::new(HashMap::new()),
                pipeline: CompositorPipeline::new(&dev),
                curr_id: AtomicUsize::new(0),
            },
            rt,
            dev: Arc::new(dev),
            toasts: Mutex::new(egui_notify::Toasts::default()),
            added_instances: Mutex::new(Vec::with_capacity(1)),
        }
    }

    pub async fn load_file(&self, path: PathBuf) -> Result<InstanceKey, SilicaError> {
        let (file, textures) =
            tokio::task::block_in_place(|| ProcreateFile::open(path, &self.dev)).unwrap();
        let mut target = CompositorTarget::new(self.dev.clone());
        target
            .data
            .flip_vertices(file.flipped.horizontally, file.flipped.vertically);
        target.set_dimensions(file.size.width, file.size.height);

        for _ in 0..file.orientation {
            target.data.rotate_vertices(true);
            target.set_dimensions(target.dim.height, target.dim.width);
        }

        let id = self
            .compositor
            .curr_id
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let key = InstanceKey(id);
        self.compositor.instances.write().insert(
            key,
            Instance {
                file: RwLock::new(file),
                target: Mutex::new(target),
                textures,
                changed: AtomicBool::new(true),
            },
        );

        Ok(key)
    }

    /// Transform tree structure of layers into a linear list of
    /// layers for rendering.
    pub fn linearize_silica_layers<'a>(
        layers: &'a crate::silica::SilicaGroup,
    ) -> Vec<CompositeLayer> {
        fn inner<'a>(
            layers: &'a crate::silica::SilicaGroup,
            composite_layers: &mut Vec<CompositeLayer>,
            mask_layer: &mut Option<(u32, &'a crate::silica::SilicaLayer)>,
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
