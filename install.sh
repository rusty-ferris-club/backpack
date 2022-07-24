#!/bin/sh
# Copyright 2019 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

set -e

if ! command -v unzip >/dev/null; then
	echo "Error: unzip is required to install Backpack." 1>&2
	exit 1
fi

if [ "$OS" = "Windows_NT" ]; then
	target="x86_64-windows"
else
	case $(uname -sm) in
	"Darwin x86_64") target="x86_64-apple-darwin" ;;
	"Darwin arm64") target="aarch64-apple-darwin" ;;
	*) target="x86_64-unknown-linux-gnu" ;;
	esac
fi

if [ $# -eq 0 ]; then
	uri="https://github.com/rusty-ferris-club/releases/latest/download/backpack-${target}.zip"
else
	uri="https://github.com/rusty-ferris-club/releases/download/${1}/backpack-${target}.zip"
fi

install="${BP_INSTALL:-$HOME/.backpack-bin}"
bin_dir="$install/bin"
exe="$bin_dir/bp"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
fi

curl --fail --location --progress-bar --output "$exe.zip" "$uri"
unzip -d "$bin_dir" -o "$exe.zip"
chmod +x "$exe"
rm "$exe.zip"

echo "Backpack was installed successfully to $exe"
if command -v bp >/dev/null; then
	echo "Run 'bp --help' to get started"
else
	case $SHELL in
	/bin/zsh) shell_profile=".zshrc" ;;
	*) shell_profile=".bashrc" ;;
	esac
	echo "Manually add the directory to your \$HOME/$shell_profile (or similar)"
	echo "  export BP_INSTALL=\"$install\""
	echo "  export PATH=\"\$BP_INSTALL/bin:\$PATH\""
	echo "Run '$exe --help' to get started"
fi
