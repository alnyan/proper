#![allow(non_camel_case_types)]
#![allow(clippy::needless_question_mark)]

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

pub mod screen_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/render/shader/screen.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Pod, Zeroable)]
        }
    }
}

pub mod screen_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/render/shader/screen.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Pod, Zeroable)]
        }
    }
}
