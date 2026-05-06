cask "douyin-downloader" do
  version "{{VERSION}}"

  on_arm do
    sha256 "{{SHA256_DMG_ARM64}}"
    url "https://github.com/anYuJia/douyin-downloader-rust/releases/download/v#{version}/Douyin.Downloader_#{version}_aarch64.dmg"
  end

  on_intel do
    sha256 "{{SHA256_DMG_X64}}"
    url "https://github.com/anYuJia/douyin-downloader-rust/releases/download/v#{version}/Douyin.Downloader_#{version}_x64.dmg"
  end

  name "Douyin Downloader"
  desc "Desktop Douyin video downloader built with Rust and Tauri"
  homepage "https://github.com/anYuJia/douyin-downloader-rust"

  app "Douyin Downloader.app"
end
