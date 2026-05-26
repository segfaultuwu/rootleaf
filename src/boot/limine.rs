use limine::BaseRevision;
use limine::request::FramebufferRequest;

#[used]
#[unsafe(link_section = ".requests")]
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

pub static MEMORY_MAP_REQUEST: limine::request::MemmapRequest =
    limine::request::MemmapRequest::new();
