//! 下载模块

#[allow(clippy::module_inception)]
pub mod downloader;

pub use downloader::{Downloader, DownloaderEvent};
