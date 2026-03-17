pub trait KnowledgeBaseProvider: Send + Sync {
    fn expand_query(&self, query: &str) -> String;
}
