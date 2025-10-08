#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Item media type is unknown")]
    UnknownMediaType,
}
