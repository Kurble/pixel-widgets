use crate::loader::Loader;
use std::sync::{Arc, Mutex};
use crate::cache::Cache;
use crate::draw::{Image, Patch};

/// Cloneable image loader
pub struct Graphics<L: Loader> {
    pub(crate) loader: Arc<L>,
    pub(crate) cache: Arc<Mutex<Cache>>,
}

impl<L: Loader> Graphics<L> {
    /// Retrieve the inner loader
    pub fn loader(&self) -> Arc<L> {
        self.loader.clone()
    }

    /// Loads an image
    pub async fn load_image<U: AsRef<str>>(&self, url: U) -> Result<Image, L::Error> {
        let bytes = self.loader.load(url).await?;
        let image = image::load_from_memory(bytes.as_slice()).unwrap();
        let image = self.cache.lock().unwrap().load_image(image.into_rgba());
        Ok(image)
    }

    /// Loads a 9 patch.
    pub async fn load_patch<U: AsRef<str>>(&self, url: U) -> Result<Patch, L::Error> {
        let bytes = self.loader.load(url).await?;
        let image = image::load_from_memory(bytes.as_slice()).unwrap();
        let image = self.cache.lock().unwrap().load_patch(image.into_rgba());
        Ok(image)
    }
}

impl<L: Loader> Clone for Graphics<L> {
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
            cache: self.cache.clone(),
        }
    }
}
