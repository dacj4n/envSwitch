class Envswitch < Formula
  desc "Fast development environment version switcher"
  homepage "https://github.com/your/envswitch"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/your/envswitch/releases/download/v#{version}/envswitch-macos-arm64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    else
      url "https://github.com/your/envswitch/releases/download/v#{version}/envswitch-macos-x64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/your/envswitch/releases/download/v#{version}/envswitch-linux-arm64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    else
      url "https://github.com/your/envswitch/releases/download/v#{version}/envswitch-linux-x64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  def install
    bin.install "envswitch-#{OS.mac? ? "macos" : "linux"}-#{Hardware::CPU.arm? ? "arm64" : "x64"}" => "envswitch"
  end

  test do
    system "#{bin}/envswitch", "--version"
  end
end
