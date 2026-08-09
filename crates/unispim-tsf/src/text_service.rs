//! TSF 文本服务核心对象。
//!
//! 对应 TSFexample 的 `TextService.cpp` / `TextService.h`：
//! 实现 `ITfTextInputProcessor` + `ITfThreadMgrEventSink` + `ITfKeyEventSink`
//! + `ITfTextEditSink` + `ITfCompositionSink`。
//!
//! 这是一个 COM 对象，通过 `#[implement]` 宏实现多个接口。
//! `TextService` 实现了 `Clone`（各字段均可克隆），供编辑会话持有时
//! 通过 `(**self).clone().into()` 将自身作为接口传递给 TSF。

use std::cell::RefCell;

use windows::core::{implement, Interface, Ref, Result as WinResult};
use windows::Win32::Foundation::{BOOL, E_FAIL, LPARAM, WPARAM};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl, ITfContext, ITfContextComposition,
    ITfDocumentMgr, ITfEditRecord, ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection,
    ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfSource, ITfTextEditSink,
    ITfTextEditSink_Impl, ITfTextInputProcessor, ITfTextInputProcessor_Impl, ITfThreadMgr,
    ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl, INSERT_TEXT_AT_SELECTION_FLAGS,
    TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_QUERYONLY, TF_SELECTION,
};

use crate::tsf::{TsfAdapter, TsfOp};

/// 文本服务。
#[derive(Clone)]
#[implement(
    ITfTextInputProcessor,
    ITfThreadMgrEventSink,
    ITfKeyEventSink,
    ITfTextEditSink,
    ITfCompositionSink
)]
pub struct TextService {
    /// 线程管理器。
    pub thread_mgr: RefCell<Option<ITfThreadMgr>>,
    /// 客户端 ID。
    pub client_id: RefCell<u32>,
    /// 当前组合。
    pub composition: RefCell<Option<ITfComposition>>,
    /// TSF 适配器（引擎）。
    pub adapter: RefCell<TsfAdapter>,
    /// ThreadMgrEventSink cookie。
    pub thread_mgr_event_sink_cookie: RefCell<u32>,
    /// TextEditSink context。
    pub text_edit_sink_context: RefCell<Option<ITfContext>>,
    /// TextEditSink cookie。
    pub text_edit_sink_cookie: RefCell<u32>,
    /// 键盘开关。
    pub keyboard_open: RefCell<bool>,
}

impl TextService {
    /// 创建新的文本服务。
    pub fn new(adapter: TsfAdapter) -> Self {
        TextService {
            thread_mgr: RefCell::new(None),
            client_id: RefCell::new(0),
            composition: RefCell::new(None),
            adapter: RefCell::new(adapter),
            thread_mgr_event_sink_cookie: RefCell::new(0),
            text_edit_sink_context: RefCell::new(None),
            text_edit_sink_cookie: RefCell::new(0),
            keyboard_open: RefCell::new(true),
        }
    }

    /// 是否正在组合。
    pub fn is_composing(&self) -> bool {
        self.composition.borrow().is_some()
    }

    /// 判断按键是否被输入法接管。
    fn is_key_eaten(&self, wparam: WPARAM) -> bool {
        if !*self.keyboard_open.borrow() {
            return false;
        }
        let vk = wparam.0 as u16;
        if (0x41..=0x5A).contains(&vk)
            || (0x30..=0x39).contains(&vk)
            || (0x60..=0x69).contains(&vk)
        {
            return true;
        }
        match vk {
            0x20 | 0x08 | 0x0D | 0x21 | 0x22 => return true,
            _ => {}
        }
        if self.is_composing() && (0x25..=0x28).contains(&vk) {
            return true;
        }
        false
    }

    /// 初始化 ThreadMgrEventSink。
    fn init_thread_mgr_event_sink(&self) -> bool {
        if let Some(tm) = self.thread_mgr.borrow().as_ref()
            && let Ok(source) = tm.cast::<ITfSource>() {
                let sink: windows::core::IUnknown = self.clone().into();
                let result = unsafe {
                    source.AdviseSink(
                        &<ITfThreadMgrEventSink as Interface>::IID,
                        &sink,
                    )
                };
                if let Ok(cookie) = result {
                    *self.thread_mgr_event_sink_cookie.borrow_mut() = cookie;
                    return true;
                }
            }
        false
    }

    /// 注销 ThreadMgrEventSink。
    fn uninit_thread_mgr_event_sink(&self) {
        if let Some(tm) = self.thread_mgr.borrow().as_ref()
            && let Ok(source) = tm.cast::<ITfSource>() {
                unsafe {
                    let _ = source.UnadviseSink(*self.thread_mgr_event_sink_cookie.borrow());
                }
            }
        *self.thread_mgr_event_sink_cookie.borrow_mut() = 0;
    }

    /// 初始化 KeyEventSink。
    fn init_key_event_sink(&self) -> bool {
        if let Some(tm) = self.thread_mgr.borrow().as_ref()
            && let Ok(km) = tm.cast::<ITfKeystrokeMgr>() {
                let sink: ITfKeyEventSink = self.clone().into();
                return unsafe {
                    km.AdviseKeyEventSink(*self.client_id.borrow(), &sink, true)
                        .is_ok()
                };
            }
        false
    }

    /// 注销 KeyEventSink。
    fn uninit_key_event_sink(&self) {
        if let Some(tm) = self.thread_mgr.borrow().as_ref()
            && let Ok(km) = tm.cast::<ITfKeystrokeMgr>() {
                unsafe {
                    let _ = km.UnadviseKeyEventSink(*self.client_id.borrow());
                }
            }
    }

    /// 初始化 TextEditSink。
    fn init_text_edit_sink(&self, doc_mgr: Option<ITfDocumentMgr>) -> bool {
        let Some(doc_mgr) = doc_mgr else {
            self.uninit_text_edit_sink();
            return true;
        };
        let got = unsafe { doc_mgr.GetTop() };
        let Ok(context) = got else {
            return false;
        };
        let Ok(source) = context.cast::<ITfSource>() else {
            return false;
        };
        let sink: windows::core::IUnknown = self.clone().into();
        let result = unsafe {
            source.AdviseSink(
                &<ITfTextEditSink as Interface>::IID,
                &sink,
            )
        };
        match result {
            Ok(cookie) => {
                *self.text_edit_sink_context.borrow_mut() = Some(context);
                *self.text_edit_sink_cookie.borrow_mut() = cookie;
                true
            }
            Err(_) => false,
        }
    }

    /// 注销 TextEditSink。
    fn uninit_text_edit_sink(&self) {
        if let Some(ctx) = self.text_edit_sink_context.borrow().as_ref()
            && let Ok(source) = ctx.cast::<ITfSource>() {
                unsafe {
                    let _ = source.UnadviseSink(*self.text_edit_sink_cookie.borrow());
                }
            }
        *self.text_edit_sink_context.borrow_mut() = None;
        *self.text_edit_sink_cookie.borrow_mut() = 0;
    }

    /// 处理按键（在编辑会话中执行）。
    fn handle_key(&self, wparam: WPARAM) {
        let vk = wparam.0 as u16;
        let ops = self.adapter.borrow_mut().handle_vk(vk);
        for op in ops {
            self.apply_op(op);
        }
    }

    /// 应用一个 TSF 操作。
    fn apply_op(&self, op: TsfOp) {
        match op {
            TsfOp::Commit(text) => self.commit_text(&text),
            TsfOp::Preedit { text, cursor } => self.set_preedit(&text, cursor),
            TsfOp::ClearPreedit => self.end_composition(),
            TsfOp::Candidates { .. } => {}
        }
    }

    /// 上屏文本。
    fn commit_text(&self, text: &str) {
        if self.is_composing() {
            self.end_composition();
        }
        let Some(tm) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let got = unsafe { tm.GetFocus().and_then(|d| d.GetTop()) };
        let Ok(context) = got else {
            return;
        };
        let session = CommitTextSession {
            text: text.encode_utf16().collect(),
            context: context.clone(),
        };
        let iface: ITfEditSession = session.into();
        let _ = self.request_edit_session_with(&context, &iface);
    }

    /// 设置预编辑文本（创建或更新组合）。
    fn set_preedit(&self, text: &str, _cursor: i32) {
        let Some(tm) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let got = unsafe { tm.GetFocus().and_then(|d| d.GetTop()) };
        let Ok(context) = got else {
            return;
        };

        if self.is_composing() {
            if let Some(comp) = self.composition.borrow().clone() {
                let session = UpdatePreeditSession {
                    text: text.encode_utf16().collect(),
                    composition: comp,
                };
                let iface: ITfEditSession = session.into();
                let _ = self.request_edit_session_with(&context, &iface);
            }
        } else if !text.is_empty() {
            let session = StartPreeditSession {
                text: text.encode_utf16().collect(),
                context: context.clone(),
                service: self.clone(),
            };
            let iface: ITfEditSession = session.into();
            let _ = self.request_edit_session_with(&context, &iface);
        }
    }

    /// 终止组合。
    fn end_composition(&self) {
        if let Some(comp) = self.composition.borrow_mut().take() {
            let session = EndCompositionSession { composition: comp };
            if let Some(tm) = self.thread_mgr.borrow().clone() {
                let got = unsafe { tm.GetFocus().and_then(|d| d.GetTop()) };
                if let Ok(context) = got {
                    let iface: ITfEditSession = session.into();
                    let _ = self.request_edit_session_with(&context, &iface);
                }
            }
        }
    }

    /// 请求编辑会话（指定上下文）。
    fn request_edit_session_with(&self, context: &ITfContext, session: &ITfEditSession) -> WinResult<()> {
        unsafe {
            let _hr = context.RequestEditSession(
                *self.client_id.borrow(),
                session,
                TF_ES_SYNC | TF_ES_READWRITE,
            )?;
        }
        Ok(())
    }
}

// ── ITfTextInputProcessor ────────────────────────────────────────────────

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32) -> WinResult<()> {
        *self.thread_mgr.borrow_mut() = ptim.cloned();
        *self.client_id.borrow_mut() = tid;

        if !self.init_thread_mgr_event_sink() {
            return Err(windows::core::Error::from(E_FAIL));
        }

        if let Some(tm) = self.thread_mgr.borrow().as_ref()
            && let Ok(doc) = unsafe { tm.GetFocus() } {
                self.init_text_edit_sink(Some(doc));
            }

        if !self.init_key_event_sink() {
            return Err(windows::core::Error::from(E_FAIL));
        }

        Ok(())
    }

    fn Deactivate(&self) -> WinResult<()> {
        self.uninit_text_edit_sink();
        self.uninit_thread_mgr_event_sink();
        self.uninit_key_event_sink();
        *self.composition.borrow_mut() = None;
        *self.thread_mgr.borrow_mut() = None;
        Ok(())
    }
}

// ── ITfThreadMgrEventSink ────────────────────────────────────────────────

impl ITfThreadMgrEventSink_Impl for TextService_Impl {
    fn OnInitDocumentMgr(&self, _pdim: Ref<'_, ITfDocumentMgr>) -> WinResult<()> {
        Ok(())
    }
    fn OnUninitDocumentMgr(&self, _pdim: Ref<'_, ITfDocumentMgr>) -> WinResult<()> {
        Ok(())
    }
    fn OnSetFocus(
        &self,
        pdimfocus: Ref<'_, ITfDocumentMgr>,
        _pdimprevfocus: Ref<'_, ITfDocumentMgr>,
    ) -> WinResult<()> {
        self.init_text_edit_sink(pdimfocus.cloned());
        Ok(())    }    fn OnPushContext(&self, _pic: Ref<'_, ITfContext>) -> WinResult<()> {
        Ok(())
    }
    fn OnPopContext(&self, _pic: Ref<'_, ITfContext>) -> WinResult<()> {
        Ok(())
    }
}

// ── ITfKeyEventSink ──────────────────────────────────────────────────────

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> WinResult<()> {
        Ok(())
    }
    fn OnTestKeyDown(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        Ok(BOOL(self.is_key_eaten(wparam) as i32))
    }
    fn OnTestKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        Ok(BOOL(0))
    }
    fn OnKeyDown(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        let eaten = self.is_key_eaten(wparam);
        if eaten {
            self.handle_key(wparam);
        }
        Ok(BOOL(eaten as i32))
    }
    fn OnKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> WinResult<BOOL> {
        Ok(BOOL(0))
    }
    fn OnPreservedKey(
        &self,
        _pic: Ref<'_, ITfContext>,
        _rguid: *const windows::core::GUID,
    ) -> WinResult<BOOL> {
        Ok(BOOL(0))
    }
}

// ── ITfTextEditSink ──────────────────────────────────────────────────────

impl ITfTextEditSink_Impl for TextService_Impl {
    fn OnEndEdit(
        &self,
        _pic: Ref<'_, ITfContext>,
        _ec_read_only: u32,
        _peditrec: Ref<'_, ITfEditRecord>,
    ) -> WinResult<()> {
        Ok(())
    }
}

// ── ITfCompositionSink ──────────────────────────────────────────────────

impl ITfCompositionSink_Impl for TextService_Impl {
    fn OnCompositionTerminated(
        &self,
        _ec_write: u32,
        _pcomposition: Ref<'_, ITfComposition>,
    ) -> WinResult<()> {
        *self.composition.borrow_mut() = None;
        Ok(())
    }
}

// ── 编辑会话 ────────────────────────────────────────────────────────────

/// 提交文本会话（在组合外插入）。
#[implement(ITfEditSession)]
pub struct CommitTextSession {
    pub text: Vec<u16>,
    pub context: ITfContext,
}

impl ITfEditSession_Impl for CommitTextSession_Impl {
    fn DoEditSession(&self, ec: u32) -> WinResult<()> {
        unsafe {
            let ins: ITfInsertAtSelection = self.context.cast()?;
            let _range = ins.InsertTextAtSelection(
                ec,
                INSERT_TEXT_AT_SELECTION_FLAGS(0),
                &self.text,
            )?;
            Ok(())
        }
    }
}

/// 开始组合会话。
#[implement(ITfEditSession)]
pub struct StartPreeditSession {
    pub text: Vec<u16>,
    pub context: ITfContext,
    pub service: TextService,
}

impl ITfEditSession_Impl for StartPreeditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> WinResult<()> {
        unsafe {
            let ins: ITfInsertAtSelection = self.context.cast()?;
            let range = ins.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
            let comp_ctx: ITfContextComposition = self.context.cast()?;
            let sink: ITfCompositionSink = self.service.clone().into();
            let composition = comp_ctx.StartComposition(ec, &range, &sink)?;
            composition.GetRange()?.SetText(ec, 0, &self.text)?;
            *self.service.composition.borrow_mut() = Some(composition);

            let selection = TF_SELECTION {
                range: std::mem::ManuallyDrop::new(Some(range)),
                style: Default::default(),
            };
            let _ = self.context.SetSelection(ec, &[selection]);
            Ok(())
        }
    }
}

/// 更新组合文本会话。
#[implement(ITfEditSession)]
pub struct UpdatePreeditSession {
    pub text: Vec<u16>,
    pub composition: ITfComposition,
}

impl ITfEditSession_Impl for UpdatePreeditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> WinResult<()> {
        unsafe {
            let range = self.composition.GetRange()?;
            range.SetText(ec, 0, &self.text)?;
            Ok(())
        }
    }
}

/// 终止组合会话。
#[implement(ITfEditSession)]
pub struct EndCompositionSession {
    pub composition: ITfComposition,
}

impl ITfEditSession_Impl for EndCompositionSession_Impl {
    fn DoEditSession(&self, ec: u32) -> WinResult<()> {
        unsafe {
            self.composition.EndComposition(ec)?;
            Ok(())
        }
    }
}
