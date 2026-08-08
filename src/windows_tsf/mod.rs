//! Windows TSF 输入法（文本服务）实现。
//!
//! 参考 TSFexample（1BasicTextService ~ 7Composition）的架构，
//! 使用 `windows` crate 0.59 实现完整的 TSF 文本服务：
//!
//! - `register.rs`：注册表 / ITfInputProcessorProfiles / ITfCategoryMgr 注册
//! - `text_service.rs`：ITfTextInputProcessor + ITfThreadMgrEventSink + ITfKeyEventSink + ITfCompositionSink
//! - `tsf.rs`：TSF 与输入法引擎的适配
//!
//! 注册/注销工具：`cargo run --bin unispim_tsf_reg -- register|unregister`

mod register;
mod text_service;
mod tsf;

pub use register::{register, unregister, CLSID_TEXT_SERVICE, GUID_PROFILE};
pub use text_service::TextService;
pub use tsf::{vk_to_key, TsfAdapter, TsfOp};
