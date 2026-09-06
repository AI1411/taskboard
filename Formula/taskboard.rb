class Taskboard < Formula
  desc "Local Kanban CLI (taskboard / tb) for Apple silicon"
  homepage "https://github.com/AI1411/taskboard"
  license "MIT"
  version "0.1.0"

  # Binary install (GitHub Release asset produced by .github/workflows/release-cli.yml).
  url "https://github.com/AI1411/taskboard/releases/download/v#{version}/taskboard-aarch64-apple-darwin.tar.gz"
  # Updated after the first v0.1.0 CLI tarball is published.
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  head "https://github.com/AI1411/taskboard.git", branch: "main"

  depends_on macos: :sonoma
  depends_on arch: :arm64
  depends_on "rust" => :build if build.head?

  def install
    if build.head?
      system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/cli"
    else
      # Tarball also contains a `tb` symlink for non-brew installs; Homebrew
      # must not link that name from the Cellar so a foreign `tb` can be skipped.
      bin.install "taskboard"
    end
  end

  def post_install
    taskboard = HOMEBREW_PREFIX/"bin/taskboard"
    tb = HOMEBREW_PREFIX/"bin/tb"
    skip = false
    if tb.exist? || tb.symlink?
      begin
        skip = tb.realpath != taskboard.realpath
      rescue
        skip = true
      end
    end
    if skip
      ohai "alias_skipped"
      puts "alias_skipped"
    elsif !tb.exist? && !tb.symlink?
      tb.make_symlink(taskboard)
    end
  end

  def caveats
    <<~EOS
      Apple silicon (arm64) on macOS 14+ is the supported release target.
      `taskboard` is always linked. `tb` is created in postinstall unless a
      foreign `tb` already exists; then postinstall prints alias_skipped.
    EOS
  end

  test do
    assert_match "taskboard", shell_output("#{bin}/taskboard --help")
  end
end
