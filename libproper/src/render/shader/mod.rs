#![allow(non_camel_case_types)]

pub mod simple_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/render/shader/scene.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Pod, Zeroable)]
        }
    }
}

pub mod simple_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/render/shader/scene.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Pod, Zeroable)]
        }
    }
}
