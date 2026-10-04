#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
	cat <<'EOF'
Usage: ./infrastructure/build/site.sh <command>

Commands:
	check    Check frontend JavaScript syntax
	build    Build the Tauri desktop installer (MSI and NSIS)
	web      Validate the public static website files
	clean    Remove generated desktop bundle output
EOF
}

command="${1:-check}"
case "$command" in
	check)
		if command -v npm >/dev/null 2>&1; then
			npm run check
		else
			node --check apps/desktop/src/main.js
		fi
		;;
	build)
		if command -v npm >/dev/null 2>&1; then
			npm run release:desktop
		else
			echo "npm is required for the desktop build." >&2
			exit 127
		fi
		;;
	web)
		test -f web/index.html
		test -f web/styles.css
		test -f web/Dockerfile
		test -f infrastructure/web/nginx.conf
		echo "Website files are present."
		;;
	clean)
		rm -rf apps/desktop/src-tauri/target/release/bundle
		echo "Desktop bundle output removed."
		;;
	*) usage; exit 64 ;;
esac
