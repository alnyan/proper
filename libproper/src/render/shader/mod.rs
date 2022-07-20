pub mod scene_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/render/shader/scene.vert"
    }
}

pub mod scene_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/render/shader/scene.frag"
    }
}
