/// In-memory vector index. Loaded from SQLite at startup.
pub struct VectorStore {
    ids: Vec<String>,
    vectors: Vec<Vec<f32>>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self { ids: vec![], vectors: vec![] }
    }

    pub fn insert(&mut self, id: String, vector: Vec<f32>) {
        self.ids.push(id);
        self.vectors.push(vector);
    }

    /// Returns (id, cosine_similarity) sorted descending.
    pub fn query(&self, query_vec: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(usize, f32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (i, cosine(v, query_vec)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
            .into_iter()
            .take(top_k)
            .map(|(i, s)| (self.ids[i].clone(), s))
            .collect()
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}
