# Homebrew formula for a tap, filled in by the release workflow (render.mjs).
class Sdc < Formula
  desc "Command-line tool for sdlang, a spec language for code written by AI agents"
  homepage "https://github.com/__REPO__"
  version "__VERSION__"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/__REPO__/releases/download/__TAG__/sdc-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256:aarch64-apple-darwin__"
    end
    on_intel do
      url "https://github.com/__REPO__/releases/download/__TAG__/sdc-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256:x86_64-apple-darwin__"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/__REPO__/releases/download/__TAG__/sdc-aarch64-unknown-linux-musl.tar.gz"
      sha256 "__SHA256:aarch64-unknown-linux-musl__"
    end
    on_intel do
      url "https://github.com/__REPO__/releases/download/__TAG__/sdc-x86_64-unknown-linux-musl.tar.gz"
      sha256 "__SHA256:x86_64-unknown-linux-musl__"
    end
  end

  def install
    bin.install "sdc"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sdc --version")
  end
end
