use std::sync::Arc;
use rafx_api::RafxError;

#[derive(Debug, Clone)]
pub enum ApiaryError {
    StringError(String),
    IoError(Arc<std::io::Error>),
    RafxError(RafxError),
    #[cfg(feature = "rafx-vulkan")]
    VkError(vk::Result),
    #[cfg(feature = "rafx-vulkan")]
    VkLoadingError(Arc<ash::LoadingError>),
    #[cfg(feature = "rafx-vulkan")]
    VkCreateInstanceError(Arc<VkCreateInstanceError>),
    #[cfg(feature = "rafx-vulkan")]
    AllocationError(Arc<gpu_allocator::AllocationError>),
    #[cfg(any(feature = "rafx-gles2", feature = "rafx-gles3"))]
    GlError(u32),
}

impl From<RafxError> for ApiaryError {
    fn from(e: RafxError) -> Self { ApiaryError::RafxError(e) }
}

pub type ApiaryResult<T> = Result<T, ApiaryError>;
