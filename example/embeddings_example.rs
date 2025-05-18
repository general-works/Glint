//! Example: Embedding model usage with Glint
//! Run with: cargo run --bin embeddings

use glint::embeddings::MockEmbeddings;
use glint::schema::Document;
use glint::traits::Runnable;
use glint::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a mock embeddings model with 384 dimensions
    let embeddings = MockEmbeddings::new(384);

    // Create a document
    let doc = Document::new("This is a test document");

    // Get embeddings for the document
    let embedding = embeddings.invoke(doc.page_content).await?;

    println!("Generated embedding: {:?}", embedding);

    Ok(())
}
