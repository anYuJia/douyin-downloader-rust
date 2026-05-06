cask "douyin-downloader" do
  version "0.0.9"

  on_arm do
    sha256 "13d23f7839b5554b5a113d07c20e7e7978d50f167ac8293c9e00523ec1ad286c"
    url "https://github.com/anYuJia/douyin-downloader-rust/releases/download/v#{version}/Douyin.Downloader_#{version}_aarch64.dmg"
  end

  on_intel do
    sha256 "378cc0d37312e92866a5bb90f2277f85a16999c5fcb390efff12fb82f2ebb0c3"
    url "https://github.com/anYuJia/douyin-downloader-rust/releases/download/v#{version}/Douyin.Downloader_#{version}_x64.dmg"
  end

  name "Douyin Downloader"
  desc "Desktop Douyin video downloader built with Rust and Tauri"
  homepage "https://github.com/anYuJia/douyin-downloader-rust"

  app "Douyin Downloader.app"
end
