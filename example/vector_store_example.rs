//! Example: Vector store usage with Glint
//! Run with: cargo run --bin vector_store

use glint::embeddings::MockEmbeddings;
use glint::schema::Document;
use glint::traits::VectorStore;
use glint::vectorstores::memory::MemoryVectorStore;
use glint::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a mock embeddings model
    let embeddings = MockEmbeddings::new(384);

    // Create a vector store
    let mut store = MemoryVectorStore::new(embeddings);

    // Add some documents
    let docs = vec![
        Document::new("Rust is a systems programming language"),
        Document::new("Python is a high-level programming language"),
        Document::new("JavaScript is a scripting language"),
    ];

    store.add_documents(docs).await?;

    // Search for similar documents
    let results = store.search("programming language", 2).await?;

    println!("Search results:");
    for (doc, score) in results {
        println!("Score: {:.2}, Content: {}", score, doc.page_content);
    }

    Ok(())
}
