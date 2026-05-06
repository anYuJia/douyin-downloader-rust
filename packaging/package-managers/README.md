# Package Manager Distribution

This directory contains starter manifests for package-manager distribution after a GitHub Release is published.

Recommended order:

1. Publish the GitHub Release and wait for all assets to upload.
2. Generate SHA-256 hashes for the installer assets.
3. Replace `{{VERSION}}` and `{{SHA256_*}}` placeholders in the manifests.
4. Submit the manifests to the target package-manager repositories.

## Asset Mapping

| Channel | Asset |
|:---|:---|
| Homebrew Cask arm64 | `Douyin.Downloader_{{VERSION}}_aarch64.dmg` |
| Homebrew Cask x64 | `Douyin.Downloader_{{VERSION}}_x64.dmg` |
| Scoop | `Douyin-Downloader_{{VERSION}}_x64_portable.exe` |
| winget installer | `Douyin.Downloader_{{VERSION}}_x64-setup.exe` |

## Hashes

Download the release assets and compute hashes:

```bash
VERSION=v0.0.9
gh release download "$VERSION" --pattern 'Douyin.Downloader_*' --pattern 'Douyin-Downloader_*'
shasum -a 256 Douyin.Downloader_*.dmg Douyin.Downloader_*_x64-setup.exe Douyin-Downloader_*_x64_portable.exe
```

Use the hash values to replace placeholders in:

- `homebrew/douyin-downloader.rb`
- `scoop/douyin-downloader.json`
- `winget/AnYuJia.DouyinDownloader/*.yaml`

Generated, version-pinned manifests are stored under `generated/<version>/` after a release has been published and hashes have been verified.

## Notes

- Homebrew Cask usually belongs in `homebrew-cask/Casks/d/douyin-downloader.rb`.
- Scoop can be maintained in a custom bucket first, then submitted to a public bucket if desired.
- winget manifests are normally submitted to `microsoft/winget-pkgs` under `manifests/a/AnYuJia/DouyinDownloader/{{VERSION}}/`.
- Store submission can be done after one stable release has been verified manually.
