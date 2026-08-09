//! 华宇拼音输入法（uniSpim）核心引擎库。
//!
//! 主要模块：
//! - [`syllable`]：音节编码、声母/韵母常量、模糊音
//! - [`syllable_map`]：拼音 -> 音节转换表（460 条）
//! - [`parse`]：拼音串解析
//! - [`wordlib`]：词库（.uwl V6.6）格式读写与查询
//! - [`ci`]：词汇候选检索
//! - [`hzdata`]：汉字数据（hzpy.dat）读取与单字候选
//! - [`symbol`]：中英文符号转换
//! - [`ime`]：输入法引擎（纯状态机）

pub mod ci;
pub mod hzdata;
pub mod ime;
pub mod parse;
pub mod syllable;
pub mod syllable_map;
pub mod symbol;
pub mod wordlib;
