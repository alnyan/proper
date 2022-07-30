use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use vulkano::{
    device::Queue,
    format::Format,
    image::{view::ImageView, ImageDimensions, ImmutableImage, MipmapsCount},
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
    sync::GpuFuture,
};

use crate::error::Error;

#[derive(Clone)]
pub struct SampledTexture {
    sampler: Arc<Sampler>,
    image: Arc<ImageView<ImmutableImage>>,
}

pub struct TextureRegistry {
    gfx_queue: Arc<Queue>,
    sampler: Arc<Sampler>,
    data: BTreeMap<String, Arc<SampledTexture>>,
}

impl TextureRegistry {
    pub fn new(gfx_queue: Arc<Queue>) -> Result<Self, Error> {
        let sampler = Sampler::new(
            gfx_queue.device().clone(),
            SamplerCreateInfo {
                min_filter: Filter::Linear,
                mag_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unwrap();

        Ok(Self {
            gfx_queue,
            sampler,
            data: BTreeMap::new(),
        })
    }

    pub fn get_or_load(&mut self, name: &str) -> Result<Arc<SampledTexture>, Error> {
        if let Some(texture) = self.data.get(name) {
            Ok(texture.clone())
        } else {
            log::info!("Loading texture {:?}", name);
            let filename = name.to_owned() + ".png";
            let mut path = PathBuf::from("res/textures");
            path.push(filename);

            let image = self.load_image(path);
            let texture = Arc::new(SampledTexture {
                sampler: self.sampler.clone(),
                image,
            });

            self.data.insert(name.to_owned(), texture.clone());

            Ok(texture)
        }
    }

    fn load_image<P: AsRef<Path>>(&self, path: P) -> Arc<ImageView<ImmutableImage>> {
        let image = image::open(path).unwrap();
        let width = image.width();
        let height = image.height();
        let data = image.into_rgba8();

        let (texture, init) = ImmutableImage::from_iter(
            data.into_raw(),
            ImageDimensions::Dim2d {
                width,
                height,
                array_layers: 1,
            },
            MipmapsCount::One,
            Format::R8G8B8A8_UNORM,
            self.gfx_queue.clone(),
        )
        .unwrap();

        init.then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        ImageView::new_default(texture).unwrap()
    }
}

impl SampledTexture {
    #[inline]
    pub const fn image(&self) -> &Arc<ImageView<ImmutableImage>> {
        &self.image
    }

    #[inline]
    pub const fn sampler(&self) -> &Arc<Sampler> {
        &self.sampler
    }
}
