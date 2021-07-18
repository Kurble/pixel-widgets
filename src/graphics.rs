use std::sync::{Arc, Mutex};

use anyhow::*;

use crate::cache::Cache;
use crate::draw::{ImageData, Patch};

/// Cloneable image loader
pub struct Graphics {
    pub(crate) cache: Arc<Mutex<Cache>>,
}

impl Graphics {
    /// Loads an image
    pub fn load_image<B: AsRef<[u8]>>(&self, bytes: B) -> Result<ImageData> {
        let image = image::load_from_memory(bytes.as_ref())?;
        let image = self.cache.lock().unwrap().load_image(image.into_rgba8());
        Ok(image)
    }

    /// Loads a 9 patch.
    pub fn load_patch<B: AsRef<[u8]>>(&self, bytes: B) -> Result<Patch> {
        let image = image::load_from_memory(bytes.as_ref())?;
        let image = self.cache.lock().unwrap().load_patch(image.into_rgba8());
        Ok(image)
    }
}

impl Clone for Graphics {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}
