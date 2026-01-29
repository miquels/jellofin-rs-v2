use image::{DynamicImage, GenericImageView, ImageFormat};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Image resizer with caching
pub struct ImageResizer {
    cache_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageResizerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Cache error: {0}")]
    Cache(String),
}

pub type Result<T> = std::result::Result<T, ImageResizerError>;

impl ImageResizer {
    /// Create a new image resizer with specified cache directory
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Resize an image and return the path to the cached result
    /// Returns original path if no resizing needed or on error
    pub fn resize_image(
        &self,
        source_path: &Path,
        width: Option<u32>,
        height: Option<u32>,
        quality: Option<u32>,
    ) -> PathBuf {
        let params = format!("w={:?} h={:?} q={:?}", width, height, quality);
        tracing::info!("Resize request for {}: {}", source_path.display(), params);

        // If no dimensions specified, return original
        if width.is_none() && height.is_none() && quality.is_none() {
            tracing::info!("No resize params for {}, returning original", source_path.display());
            return source_path.to_path_buf();
        }

        // Generate cache key
        let cache_key = self.generate_cache_key(source_path, width, height, quality);

        // Sharding logic: use first 2 chars of cache key
        if cache_key.len() < 2 {
            tracing::error!("Cache key too short: {}", cache_key);
            return source_path.to_path_buf();
        }

        let prefix = &cache_key[0..2];
        let shard_dir = self.cache_dir.join(prefix);
        let cache_path = shard_dir.join(&cache_key);

        // Check if cached version exists
        if cache_path.exists() {
            tracing::info!("Cache hit: {}", cache_path.display());
            return cache_path;
        }

        // Ensure shard directory exists
        if let Err(e) = fs::create_dir_all(&shard_dir) {
            tracing::error!("Failed to create cache shard directory {}: {}", shard_dir.display(), e);
            return source_path.to_path_buf(); // Return original if we can't allow caching
        }

        tracing::info!("Resizing/caching: {} -> {}", source_path.display(), cache_path.display());

        // Resize and cache
        match self.resize_and_cache(source_path, &cache_path, width, height, quality) {
            Ok(()) => {
                tracing::info!("Resize success: {}", cache_path.display());
                cache_path
            }
            Err(e) => {
                tracing::error!("Failed to resize image {}: {}", source_path.display(), e);
                // Attempt to cleanup partial file if needed, though File::create truncates
                source_path.to_path_buf()
            }
        }
    }

    /// Generate cache key from source path and parameters
    fn generate_cache_key(
        &self,
        source_path: &Path,
        width: Option<u32>,
        height: Option<u32>,
        quality: Option<u32>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source_path.to_string_lossy().as_bytes());
        hasher.update(format!("{:?}x{:?}q{:?}", width, height, quality).as_bytes());
        let hash = hasher.finalize();

        // Get file extension
        let ext = source_path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");

        format!("{:x}.{}", hash, ext)
    }

    /// Resize image and save to cache
    fn resize_and_cache(
        &self,
        source_path: &Path,
        cache_path: &Path,
        width: Option<u32>,
        height: Option<u32>,
        quality: Option<u32>,
    ) -> Result<()> {
        // Load image
        let img = image::open(source_path)?;

        // Calculate dimensions
        let (target_width, target_height) = self.calculate_dimensions(&img, width, height);

        // Resize
        let resized = img.resize(target_width, target_height, image::imageops::FilterType::Lanczos3);

        // Determine format
        let format = self.detect_format(source_path)?;

        // Save with quality
        match format {
            ImageFormat::Jpeg => {
                let quality = quality.unwrap_or(90).min(100) as u8;
                let mut output = fs::File::create(cache_path)?;
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
                resized.write_with_encoder(encoder)?;
            }
            ImageFormat::Png => {
                resized.save_with_format(cache_path, ImageFormat::Png)?;
            }
            ImageFormat::WebP => {
                resized.save_with_format(cache_path, ImageFormat::WebP)?;
            }
            _ => {
                resized.save(cache_path)?;
            }
        }

        Ok(())
    }

    /// Calculate target dimensions maintaining aspect ratio
    fn calculate_dimensions(&self, img: &DynamicImage, width: Option<u32>, height: Option<u32>) -> (u32, u32) {
        let (orig_width, orig_height) = img.dimensions();

        match (width, height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let ratio = orig_height as f32 / orig_width as f32;
                (w, (w as f32 * ratio) as u32)
            }
            (None, Some(h)) => {
                let ratio = orig_width as f32 / orig_height as f32;
                ((h as f32 * ratio) as u32, h)
            }
            (None, None) => (orig_width, orig_height),
        }
    }

    /// Detect image format from file content
    fn detect_format(&self, path: &Path) -> Result<ImageFormat> {
        let mut file = fs::File::open(path)?;
        let mut buffer = [0; 512];
        std::io::Read::read(&mut file, &mut buffer)?;

        Ok(image::guess_format(&buffer)?)
    }

    /// Clear cache directory
    pub fn clear_cache(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    /// Get cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_new_resizer() {
        let temp_dir = env::temp_dir().join("test_image_cache");
        let resizer = ImageResizer::new(temp_dir.clone());
        assert!(resizer.is_ok());
        assert!(temp_dir.exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_generate_cache_key() {
        let temp_dir = env::temp_dir().join("test_cache");
        let resizer = ImageResizer::new(temp_dir.clone()).unwrap();

        let path = Path::new("/test/image.jpg");
        let key1 = resizer.generate_cache_key(path, Some(100), Some(100), Some(90));
        let key2 = resizer.generate_cache_key(path, Some(100), Some(100), Some(90));
        let key3 = resizer.generate_cache_key(path, Some(200), Some(200), Some(90));

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_calculate_dimensions() {
        let temp_dir = env::temp_dir().join("test_dims");
        let resizer = ImageResizer::new(temp_dir.clone()).unwrap();

        let img = DynamicImage::new_rgb8(800, 600);

        // Width only
        let (w, h) = resizer.calculate_dimensions(&img, Some(400), None);
        assert_eq!(w, 400);
        assert_eq!(h, 300);

        // Height only
        let (w, h) = resizer.calculate_dimensions(&img, None, Some(300));
        assert_eq!(w, 400);
        assert_eq!(h, 300);

        // Both specified
        let (w, h) = resizer.calculate_dimensions(&img, Some(100), Some(100));
        assert_eq!(w, 100);
        assert_eq!(h, 100);

        let _ = fs::remove_dir_all(temp_dir);
    }
}
