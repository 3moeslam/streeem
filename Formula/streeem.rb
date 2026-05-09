class Streeem < Formula
  desc "Rust TUI that hosts multiple interactive terminals in a staggered grid"
  homepage "https://github.com/3moeslam/streeem"
  version "0.2.4"
  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/3moeslam/streeem/releases/download/v0.2.4/streeem-aarch64-apple-darwin.tar.xz"
      sha256 "79ef0d1b09dc577ab7e7ede658b4755f4353262564082634705ae41241172c5d"
    end
    if Hardware::CPU.intel?
      url "https://github.com/3moeslam/streeem/releases/download/v0.2.4/streeem-x86_64-apple-darwin.tar.xz"
      sha256 "5b612eadb603386b5d43503324a66a9908302714f696082c472ec2e0c7b2869c"
    end
  end
  license any_of: ["MIT", "Apache-2.0"]

  BINARY_ALIASES = {
    "aarch64-apple-darwin": {},
    "x86_64-apple-darwin":  {},
  }.freeze

  def target_triple
    cpu = Hardware::CPU.arm? ? "aarch64" : "x86_64"
    os = OS.mac? ? "apple-darwin" : "unknown-linux-gnu"

    "#{cpu}-#{os}"
  end

  def install_binary_aliases!
    BINARY_ALIASES[target_triple.to_sym].each do |source, dests|
      dests.each do |dest|
        bin.install_symlink bin/source.to_s => dest
      end
    end
  end

  def install
    bin.install "streeem" if OS.mac? && Hardware::CPU.arm?
    bin.install "streeem" if OS.mac? && Hardware::CPU.intel?

    install_binary_aliases!

    # Homebrew will automatically install these, so we don't need to do that
    doc_files = Dir["README.*", "readme.*", "LICENSE", "LICENSE.*", "CHANGELOG.*"]
    leftover_contents = Dir["*"] - doc_files

    # Install any leftover files in pkgshare; these are probably config or
    # sample files.
    pkgshare.install(*leftover_contents) unless leftover_contents.empty?
  end
end
