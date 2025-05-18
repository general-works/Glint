/// Struct to define chunk size constraints
#[derive(Debug, Clone, Copy)]
pub struct ChunkSize {
    /// The target size for each chunk
    pub chunk_size: usize,
    /// The amount of overlap between chunks
    pub chunk_overlap: usize,
}

impl ChunkSize {
    /// Create a new chunk size configuration
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        if chunk_overlap >= chunk_size {
            panic!("Chunk overlap must be less than chunk size");
        }
        Self {
            chunk_size,
            chunk_overlap,
        }
    }
}

impl Default for ChunkSize {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
        }
    }
}
