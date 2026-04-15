use crate::db::repo::{TagCategory, TagRecord};

#[derive(Debug, Clone)]
pub struct QueryNormalization {
    pub tag_query: String,
    pub expanded_query: String,
}

pub trait KnowledgeBaseProvider: Send + Sync {
    fn expand_query(&self, query: &str) -> String;
    fn normalize_query(&self, query: &str) -> QueryNormalization;
    fn related_terms(&self, query: &str) -> Vec<String>;
    fn auto_tag(&self, ocr_text: &str, file_name: &str) -> Vec<TagRecord>;
}

pub fn category_threshold(category: &TagCategory) -> f32 {
    match category {
        TagCategory::Meme => 0.6,
        TagCategory::Source => 0.7,
        TagCategory::Person => 0.8,
        TagCategory::Custom => 1.0,
    }
}
