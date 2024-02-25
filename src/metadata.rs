#[derive(Clone, Debug)]
pub enum Metadata {
    Article {
        title: String,
        description: Option<String>,
        author: Option<String>,
        tags: Vec<String>,

        modified: chrono::DateTime<chrono::Utc>,
        // created: chrono::DateTime<chrono::Utc>,

        url: String,
    },
    Image {
        url: String,
    },
}
