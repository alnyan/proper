use std::{collections::BTreeMap, mem::MaybeUninit, sync::Arc};

use nalgebra::Point4;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, PrimaryCommandBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    image::{view::ImageView, ImmutableImage},
    pipeline::{
        graphics::{
            depth_stencil::DepthStencilState,
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{RenderPass, Subpass},
    sampler::Sampler,
    shader::ShaderModule,
    sync::GpuFuture,
};

use crate::render::{shader, Vertex};

pub trait MaterialTemplate {
    fn recreate_pipeline(
        &mut self,
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: &Viewport,
    );

    fn pipeline(&self) -> &Arc<GraphicsPipeline>;
    fn create_instance(
        &self,
        gfx_queue: Arc<Queue>,
        create_info: MaterialInstanceCreateInfo,
    ) -> (MaterialInstance, Box<dyn GpuFuture>);
}

#[derive(Clone)]
pub struct SampledImage {
    image: Arc<ImageView<ImmutableImage>>,
    sampler: Arc<Sampler>,
}

#[derive(Clone)]
pub struct MaterialInstanceCreateInfo {
    textures: BTreeMap<String, SampledImage>,
    colors: BTreeMap<String, [f32; 4]>,
}

pub struct MaterialInstance {
    set_index: u32,
    material_set: Arc<PersistentDescriptorSet>,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialTemplateId(usize);

pub struct MaterialRegistry {
    data: Vec<Box<dyn MaterialTemplate>>,
    names: BTreeMap<String, MaterialTemplateId>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self {
            data: vec![],
            names: BTreeMap::new(),
        }
    }

    pub fn recreate_pipelines(
        &mut self,
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: &Viewport,
    ) {
        for mat in self.data.iter_mut() {
            mat.recreate_pipeline(gfx_queue, render_pass, viewport);
        }
    }

    pub fn add(&mut self, name: &str, mat: Box<dyn MaterialTemplate>) -> MaterialTemplateId {
        let id = MaterialTemplateId(self.data.len());
        self.names.insert(name.to_owned(), id);
        self.data.push(mat);
        id
    }

    pub fn get_id(&self, name: &str) -> Option<MaterialTemplateId> {
        self.names.get(name).cloned()
    }

    pub fn get(&self, id: MaterialTemplateId) -> &Box<dyn MaterialTemplate> {
        &self.data[id.0]
    }
}

impl MaterialInstance {
    pub fn bind_data(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
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
}

impl Default for MaterialInstanceCreateInfo {
    fn default() -> Self {
        Self {
            textures: BTreeMap::new(),
            colors: BTreeMap::new(),
        }
    }
}

// Specific materials

pub struct SimpleMaterial {
    pipeline: Arc<GraphicsPipeline>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
}

impl SimpleMaterial {
    pub fn new(gfx_queue: &Arc<Queue>, render_pass: &Arc<RenderPass>, viewport: &Viewport) -> Self {
        let vs = shader::simple_vs::load(gfx_queue.device().clone()).unwrap();
        let fs = shader::simple_fs::load(gfx_queue.device().clone()).unwrap();
        let pipeline = Self::create_pipeline(gfx_queue, render_pass, viewport.clone(), &vs, &fs);

        Self { pipeline, vs, fs }
    }

    fn create_pipeline(
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: Viewport,
        vs: &Arc<ShaderModule>,
        fs: &Arc<ShaderModule>,
    ) -> Arc<GraphicsPipeline> {
        GraphicsPipeline::start()
            .input_assembly_state(InputAssemblyState::new())
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(gfx_queue.device().clone())
            .unwrap()
    }
}

impl MaterialTemplate for SimpleMaterial {
    fn recreate_pipeline(
        &mut self,
        gfx_queue: &Arc<Queue>,
        render_pass: &Arc<RenderPass>,
        viewport: &Viewport,
    ) {
        self.pipeline =
            Self::create_pipeline(gfx_queue, render_pass, viewport.clone(), &self.vs, &self.fs);
    }

    fn create_instance(
        &self,
        gfx_queue: Arc<Queue>,
        create_info: MaterialInstanceCreateInfo,
    ) -> (MaterialInstance, Box<dyn GpuFuture>) {
        let (buffer, init) = ImmutableBuffer::from_data(
            shader::simple_fs::ty::Material_Data {
                diffuse_color: *create_info.colors.get("diffuse_color").unwrap(),
            },
            BufferUsage::uniform_buffer(),
            gfx_queue,
        )
        .unwrap();

        let layout = self.pipeline.layout().set_layouts().get(1).unwrap();
        let material_set = PersistentDescriptorSet::new(
            layout.clone(),
            vec![WriteDescriptorSet::buffer(0, buffer)],
        )
        .unwrap();

        (
            MaterialInstance {
                set_index: 1,
                material_set,
            },
            Box::new(init),
        )
    }

    fn pipeline(&self) -> &Arc<GraphicsPipeline> {
        &self.pipeline
    }
}