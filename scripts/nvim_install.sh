#!/usr/bin/env bash
#
# Build the tree-sitter-virdant parser and install it, along with the
# virdant.nvim plugin, into the user's Neovim config (~/.config/nvim).
#
# Layout after install:
#   ~/.config/nvim/parser/virdant.so            (tree-sitter parser)
#   ~/.config/nvim/lua/virdant/**               (from virdant.nvim/lua)
#   ~/.config/nvim/plugin/virdant.lua           (from virdant.nvim/plugin)
#   ~/.config/nvim/queries/virdant/*.scm        (from virdant.nvim/queries)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(dirname "$SCRIPT_DIR")"

TS_DIR="$WORKSPACE_DIR/tree-sitter-virdant"
PLUGIN_DIR="$WORKSPACE_DIR/virdant.nvim"
NVIM_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/nvim"

echo "==> Building tree-sitter-virdant"
(cd "$TS_DIR" && tree-sitter build -o virdant.so)

echo "==> Installing parser to $NVIM_DIR/parser/virdant.so"
mkdir -p "$NVIM_DIR/parser"
cp "$TS_DIR/virdant.so" "$NVIM_DIR/parser/virdant.so"

echo "==> Installing virdant.nvim into $NVIM_DIR"
for dir in lua plugin queries; do
    if [ -d "$PLUGIN_DIR/$dir" ]; then
        mkdir -p "$NVIM_DIR/$dir"
        cp -r "$PLUGIN_DIR/$dir/." "$NVIM_DIR/$dir/"
    fi
done

echo "==> Done."
