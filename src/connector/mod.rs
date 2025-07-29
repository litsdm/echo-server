pub mod backblaze;
pub mod mistral;
pub mod modal;

#[allow(dead_code)]
#[derive(Debug, Default)]
pub enum HttpMethod {
    #[default]
    Post,
    Get,
    Put,
    Delete,
}
