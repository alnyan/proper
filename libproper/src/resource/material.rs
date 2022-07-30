use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    command_buffer::{AutoCommandBufferBuilder, SecondaryAutoCommandBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    pipeline::{
        graphics::{
            depth_stencil::DepthStencilState,
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{RenderPass, Subpass},
    shader::ShaderModule,
    sync::GpuFuture,
};

use crate::{
    error::Error,
    render::{shader, Vertex},
};

use super::texture::SampledTexture;

pub trait MaterialTemplate: Send + Sync {
    fn recreate_pipeline(
        &self,
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: &Viewport,
    ) -> Result<(), Error>;

    fn pipeline(&self) -> &RwLock<Arc<GraphicsPipeline>>;
    fn create_instance(
        &self,
        gfx_queue: Arc<Queue>,
        create_info: MaterialInstanceCreateInfo,
    ) -> Result<(MaterialInstance, Box<dyn GpuFuture>), Error>;

    fn id(&self) -> &AtomicU64;
}

#[derive(Clone, Default)]
pub struct MaterialInstanceCreateInfo {
    textures: BTreeMap<String, Arc<SampledTexture>>,
    colors: BTreeMap<String, [f32; 4]>,
}

pub struct MaterialInstance {
    set_index: u32,
    material_set: Arc<PersistentDescriptorSet>,
}

pub struct MaterialRegistry {
    gfx_queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
    last_id: u64,
    data: BTreeMap<String, Arc<dyn MaterialTemplate>>,
}

unsafe impl Send for MaterialRegistry {}

impl MaterialRegistry {
    pub fn new(gfx_queue: Arc<Queue>, render_pass: Arc<RenderPass>, viewport: Viewport) -> Self {
        Self {
            gfx_queue,
            render_pass,
            viewport,
            last_id: 0,
            data: BTreeMap::new(),
        }
    }

    pub fn get_or_load(&mut self, name: &str) -> Result<Arc<dyn MaterialTemplate>, Error> {
        if let Some(template) = self.get(name) {
            Ok(template.clone())
        } else {
            self.last_id += 1;
            let id = self.last_id;
            log::info!("Loading material {:?} (#{})", name, id);

            let mat = match name {
                "simple" => Arc::new(
                    SimpleMaterial::new(&self.gfx_queue, &self.render_pass, &self.viewport)
                        .unwrap(),
                ),
                _ => panic!(),
            };

            mat.id().store(id, Ordering::Release);

            self.data.insert(name.to_owned(), mat.clone());

            Ok(mat)
        }
    }

    pub fn recreate_pipelines(&mut self, viewport: &Viewport) -> Result<(), Error> {
        self.viewport = viewport.clone();
        for mat in self.data.values_mut() {
            mat.recreate_pipeline(&self.gfx_queue, &self.render_pass, viewport)?;
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn MaterialTemplate>> {
        self.data.get(name)
    }
}

impl MaterialInstance {
    pub fn bind_data(
        &self,
        builder: &mut AutoCommandBufferBuilder<SecondaryAutoCommandBuffer>,
        pipeline: &Arc<GraphicsPipeline>,
    ) {
        builder.bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            pipeline.layout().clone(),
            self.set_index,
            self.material_set.clone(),
        );
    }
}

impl MaterialInstanceCreateInfo {
    pub fn with_color(mut self, name: &str, color: [f32; 4]) -> Self {
        self.colors.insert(name.to_owned(), color);
        self
    }

    pub fn with_texture(mut self, name: &str, texture: Arc<SampledTexture>) -> Self {
        self.textures.insert(name.to_owned(), texture);
        self
    }
}

// Specific materials

pub struct SimpleMaterial {
    pipeline: RwLock<Arc<GraphicsPipeline>>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    id: AtomicU64,
}

impl SimpleMaterial {
    pub fn new(
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: &Viewport,
    ) -> Result<Self, Error> {
        let vs = shader::simple_vs::load(gfx_queue.device().clone())?;
        let fs = shader::simple_fs::load(gfx_queue.device().clone())?;
        let pipeline = RwLock::new(Self::create_pipeline(
            gfx_queue,
            render_pass,
            viewport.clone(),
            &vs,
            &fs,
        )?);

        Ok(Self {
            pipeline,
            vs,
            fs,
            id: AtomicU64::new(0),
        })
    }

    fn create_pipeline(
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: Viewport,
        vs: &Arc<ShaderModule>,
        fs: &Arc<ShaderModule>,
    ) -> Result<Arc<GraphicsPipeline>, Error> {
        let subpass = Subpass::from(render_pass.clone(), 0).ok_or(Error::MissingSubpass)?;

        GraphicsPipeline::start()
            .input_assembly_state(InputAssemblyState::new())
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(
                vs.entry_point("main")
                    .ok_or(Error::MissingShaderEntryPoint)?,
                (),
            )
            .fragment_shader(
                fs.entry_point("main")
                    .ok_or(Error::MissingShaderEntryPoint)?,
                (),
            )
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .multisample_state(MultisampleState {
                rasterization_samples: subpass.num_samples().unwrap(),
                ..Default::default()
            })
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .render_pass(subpass)
            .build(gfx_queue.device().clone())
            .map_err(Error::from)
    }
}

impl MaterialTemplate for SimpleMaterial {
    fn id(&self) -> &AtomicU64 {
        &self.id
    }

    fn recreate_pipeline(
        &self,
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: &Viewport,
    ) -> Result<(), Error> {
        let mut lock = self.pipeline.write().unwrap();
        *lock =
            Self::create_pipeline(gfx_queue, render_pass, viewport.clone(), &self.vs, &self.fs)?;
        Ok(())
    }

    fn create_instance(
        &self,
        gfx_queue: Arc<Queue>,
        create_info: MaterialInstanceCreateInfo,
    ) -> Result<(MaterialInstance, Box<dyn GpuFuture>), Error> {
        let (buffer, init) = ImmutableBuffer::from_data(
            shader::simple_fs::ty::Material_Data {
                diffuse_color: *create_info.colors.get("diffuse_color").unwrap_or(&[1.0; 4]),
            },
            BufferUsage::uniform_buffer(),
            gfx_queue,
        )?;

        let diffuse_map;
        if let Some(map) = create_info.textures.get("diffuse_map") {
            diffuse_map = WriteDescriptorSet::image_view_sampler(
                1,
                map.image().clone(),
                map.sampler().clone(),
            );
        } else {
            diffuse_map = WriteDescriptorSet::none(1);
        }

        let pipeline_lock = self.pipeline.read().unwrap();
        let layout = pipeline_lock.layout().set_layouts().get(1).unwrap();
        let material_set = PersistentDescriptorSet::new(
            layout.clone(),
            vec![WriteDescriptorSet::buffer(0, buffer), diffuse_map],
        )?;

        Ok((
            MaterialInstance {
                set_index: 1,
                material_set,
            },
            Box::new(init),
        ))
    }

    fn pipeline(&self) -> &RwLock<Arc<GraphicsPipeline>> {
        &self.pipeline
    }
}
