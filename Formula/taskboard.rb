class Taskboard < Formula
  desc "Local Kanban CLI (taskboard / tb) for Apple silicon"
  homepage "https://github.com/AI1411/taskboard"
  license "MIT"
  version "0.1.1"

  # Binary install (GitHub Release asset produced by .github/workflows/release-cli.yml).
  url "https://github.com/AI1411/taskboard/releases/download/v#{version}/taskboard-aarch64-apple-darwin.tar.gz"
  sha256 "ab06fbe9a7e5c09e51add912813d6962766844fb6f16b1abad5c58bbb4940a39"

  head "https://github.com/AI1411/taskboard.git", branch: "main"

  depends_on macos: :sonoma
  depends_on arch: :arm64
  depends_on "rust" => :build if build.head?
  depends_on "node" => :build if build.head?
  depends_on "pnpm" => :build if build.head?

  def install
    if build.head?
      system "pnpm", "install", "--frozen-lockfile"
      system "pnpm", "--filter", "web", "build"
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

      Local MCP (stdio, not remote): copy packaging/mcp/cursor.mcp.json to
      Cursor `.cursor/mcp.json`, packaging/mcp/claude.mcp.json to Claude
      `.mcp.json`, or packaging/mcp/codex.config.toml into Codex
      `~/.codex/config.toml`. Each snippet is command tb (or taskboard),
      args ["mcp"], env TASKBOARD_ACTOR=cursor|claude|codex.

      Smoke: tools/list must include next, review, and run_cancel:
        printf '%s\\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | tb mcp

      HEAD installs build the web UI (node, pnpm, then cargo) so `tb serve`
      is not a blank page. Release bottles embed that UI.
    EOS
  end

  test do
    assert_match "taskboard", shell_output("#{bin}/taskboard --help")
  end
end
