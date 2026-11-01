#!/usr/bin/env bash
# Exercise the installed macOS WebKit; Safari remote automation is not required.
set -euo pipefail
cd "$(dirname "$0")"
probe=$(mktemp -d "${TMPDIR:-/tmp}/kui-webkit.XXXXXXXX")
trap 'rm -rf "$probe"' EXIT
xcrun clang -fobjc-arc -framework Cocoa -framework WebKit webkit/navigation.m -o "$probe/check"
"$probe/check" webkit/navigation.js "${1:-http://127.0.0.1:4321}/kui/components/button/"
