use async_trait::async_trait;
use dashmap::DashMap;

/// Vector store provider abstraction. Do not hardcode a vendor.
#[async_trait]
pub trait VectorStoreProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn upsert(&self, id: &str, embedding: Vec<f32>, text: &str);
    async fn search(&self, embedding: &[f32], limit: usize) -> Vec<(String, f32)>;
    async fn delete(&self, id: &str);
}

/// Local in-process store. Not Qdrant/Pinecone/Milvus.
pub struct LocalVectorStore {
    rows: DashMap<String, (Vec<f32>, String)>,
}

impl Default for LocalVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalVectorStore {
    pub fn new() -> Self {
        Self {
            rows: DashMap::new(),
        }
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

#[async_trait]
impl VectorStoreProvider for LocalVectorStore {
    fn name(&self) -> &str {
        "local"
    }

    async fn upsert(&self, id: &str, embedding: Vec<f32>, text: &str) {
        self.rows
            .insert(id.to_string(), (embedding, text.to_string()));
    }

    async fn search(&self, embedding: &[f32], limit: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .rows
            .iter()
            .map(|e| {
                let score = cosine(embedding, &e.value().0);
                (e.key().clone(), score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    async fn delete(&self, id: &str) {
        self.rows.remove(id);
    }
}

/// Deterministic local embedding. Not a hosted model.
pub fn local_embed(text: &str, dims: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; dims.max(8)];
    for (i, b) in text.bytes().enumerate() {
        let idx = (b as usize + i) % v.len();
        v[idx] += 1.0;
    }
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in &mut v {
            *x /= n;
        }
    }
    v
}
