use thiserror::Error as TError;
use vulkano::{
    buffer::{cpu_access::WriteLockError, immutable::ImmutableBufferCreationError},
    command_buffer::{
        BuildError, CommandBufferBeginError, CommandBufferExecError, DrawError, RenderPassError,
    },
    descriptor_set::{layout::DescriptorSetLayoutCreationError, DescriptorSetCreationError},
    device::{physical::SurfacePropertiesError, DeviceCreationError},
    image::{view::ImageViewCreationError, ImageCreationError},
    instance::InstanceCreationError,
    memory::DeviceMemoryAllocationError,
    pipeline::{graphics::GraphicsPipelineCreationError, layout::PipelineLayoutCreationError},
    render_pass::{FramebufferCreationError, RenderPassCreationError},
    shader::ShaderCreationError,
    swapchain::{AcquireError, SwapchainCreationError},
    sync::FlushError,
};

#[derive(TError, Debug)]
pub enum Error {
    #[error("Failed to create Vulkan instance")]
    InstanceCreation(#[from] InstanceCreationError),
    #[error("Failed to create Vulkan surface")]
    SurfaceCreation(#[from] vulkano_win::CreationError),
    #[error("Failed to create Vulkan device")]
    DeviceCreation(#[from] DeviceCreationError),
    #[error("Failed to create Vulkan swapchain")]
    SwapchainCreation(#[from] SwapchainCreationError),
    #[error("Failed to create Vulkan ImageView")]
    ImageViewCreation(#[from] ImageViewCreationError),
    #[error("Failed to get surface properties")]
    SurfaceProperties(#[from] SurfacePropertiesError),
    #[error("Failed to select Vulkan physical device")]
    NoPhysicalDevice,

    #[error("Queue flush failed")]
    Flush(#[from] FlushError),
    #[error("Failed to acquire swapchain image")]
    SwapchainAcquire(#[from] AcquireError),

    #[error("Command buffer execution error")]
    CommandBufferExecution(#[from] CommandBufferExecError),
    #[error("Failed to create descriptor set")]
    DescriptorSetCreation(#[from] DescriptorSetCreationError),
    #[error("Failed to build command buffer")]
    CommandBufferConstruction(#[from] BuildError),
    #[error("Failed to create Vulkan render pass")]
    RenderPassCreation(#[from] RenderPassCreationError),
    #[error("Failed to enter/leave render pass")]
    RenderPassOperatoin(#[from] RenderPassError),
    #[error("Draw command error")]
    DrawOperation(#[from] DrawError),
    #[error("Failed to allocate device memory")]
    DeviceMemoryAllocation(#[from] DeviceMemoryAllocationError),
    #[error("Failed to begin command buffer")]
    CommandBufferBegin(#[from] CommandBufferBeginError),

    #[error("Failed to create descriptor set layout")]
    DescriptorSetLayoutCreation(#[from] DescriptorSetLayoutCreationError),

    #[error("Missing shader entry point")]
    MissingShaderEntryPoint,
    #[error("Missing render subpass")]
    MissingSubpass,
    #[error("Failed to load shader")]
    ShaderLoad(#[from] ShaderCreationError),
    #[error("Failed to create graphics pipeline")]
    GraphicsPipelineCreation(#[from] GraphicsPipelineCreationError),
    #[error("Failed to create pipeline layout")]
    PipelineLayoutCreation(#[from] PipelineLayoutCreationError),
    #[error("Failed to create image")]
    ImageCreation(#[from] ImageCreationError),
    #[error("Failed to create framebuffer")]
    FramebufferCreation(#[from] FramebufferCreationError),
    #[error("Failed to create device-local buffer")]
    DeviceLocalBufferCreation(#[from] ImmutableBufferCreationError),

    #[error("Failed to acquire buffer write lock")]
    BufferWriteLock(#[from] WriteLockError),

    #[error("Resource is already loaded")]
    AlreadyLoaded,
}
