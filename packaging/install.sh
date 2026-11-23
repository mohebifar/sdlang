#!/bin/sh
# Installs sdc from a GitHub release on macOS or Linux:
#   curl -fsSL https://github.com/__REPO__/releases/download/__TAG__/sdc-installer.sh | sh
# SDC_INSTALL_DIR picks the directory (default ~/.local/bin). The release workflow fills in
# the repository and tag; SDC_REPO and SDC_TAG override them.
set -eu

repo="${SDC_REPO:-__REPO__}"
tag="${SDC_TAG:-__TAG__}"
dir="${SDC_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
  echo "sdc installer: $*" >&2
  exit 1
}

case "$repo$tag" in
  *__*) fail "set SDC_REPO=owner/name and SDC_TAG=vX.Y.Z, or use the installer attached to a release" ;;
esac

case "$(uname -s)" in
  Darwin) os=apple-darwin ;;
  Linux) os=unknown-linux-musl ;;
  *) fail "unsupported system $(uname -s); on Windows use sdc-installer.ps1" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) fail "unsupported processor $(uname -m)" ;;
esac
# A shell running under Rosetta reports x86_64 on an Apple silicon Mac.
if [ "$os" = apple-darwin ] && [ "$arch" = x86_64 ] &&
  [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = 1 ]; then
  arch=aarch64
fi

archive="sdc-$arch-$os.tar.gz"
url="https://github.com/$repo/releases/download/$tag/$archive"

download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --proto '=https' --tlsv1.2 -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1"
  else
    fail "needs curl or wget"
  fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "Downloading $url"
download "$url" "$tmp/$archive" || fail "cannot download $url"
download "$url.sha256" "$tmp/$archive.sha256" || fail "cannot download $url.sha256"

expected="$(cut -d ' ' -f 1 <"$tmp/$archive.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$archive" | cut -d ' ' -f 1)"
else
  actual="$(shasum -a 256 "$tmp/$archive" | cut -d ' ' -f 1)"
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch for $archive"

tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$dir"
cp "$tmp/sdc" "$dir/sdc"
chmod 755 "$dir/sdc"
echo "Installed $("$dir/sdc" --version) to $dir/sdc"

case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add $dir to your PATH to run sdc." ;;
esac
