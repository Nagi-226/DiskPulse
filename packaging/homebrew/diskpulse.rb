cask "diskpulse" do
  version "0.9.1"
  sha256 :no_check

  url "https://github.com/Nagi-226/DiskPulse/releases/download/v0.9.1/DiskPulse_0.9.1_x64.dmg",
      verified: "github.com/Nagi-226/DiskPulse/"
  name "DiskPulse"
  desc "Real-time disk space monitor and safe cleanup tool"
  homepage "https://github.com/Nagi-226/DiskPulse"

  app "DiskPulse.app"

  zap trash: [
    "~/Library/Application Support/com.fjl03.diskpulse",
    "~/Library/Preferences/com.fjl03.diskpulse.plist",
    "~/Library/Saved Application State/com.fjl03.diskpulse.savedState",
  ]

  caveats <<~EOS
    DiskPulse's first Homebrew Cask release may temporarily use an unsigned build
    until Apple Developer ID notarization is available. If macOS Gatekeeper blocks
    launch, right-click DiskPulse.app and choose Open, or install from a signed
    GitHub release once published.
  EOS
end

