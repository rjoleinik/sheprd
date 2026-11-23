#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    // keep-sorted start
    #[cfg(target_os = "macos")]
    #[error("the file preview is not a valid image")]
    Decode(#[from] image::ImageError),
    #[cfg(target_os = "macos")]
    #[error("the file preview could not be prepared")]
    Io(#[from] std::io::Error),
    #[cfg(target_os = "macos")]
    #[error("the file preview exceeded its limits")]
    Limit,
    #[error("the file preview is unavailable")]
    Unavailable,
    // keep-sorted end
}
