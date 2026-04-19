#!/usr/bin/env bash
#
# RePortal installer for macOS and Linux.
#
# Downloads the latest (or pinned) reportal release archive from GitHub,
# installs the `reportal` and `rep` binaries into ~/.local/bin, and appends
# an idempotent `eval "$(rep init <shell>)"` block to the user's shell rc
# file. Matches the distribution pattern used by starship/mise/zoxide.
#
# Usage:
#   curl --proto '=https' --tlsv1.2 -LsSf \
#     https://github.com/CheckPickerUpper/reportal/releases/latest/download/reportal-installer.sh \
#     | bash
#
# Optional env:
#   REPORTAL_VERSION=v0.15.0   Pin to a specific tag instead of "latest".
#
# Licensed under MIT.

set -euo pipefail

readonly REPO="CheckPickerUpper/reportal"
readonly API_LATEST="https://api.github.com/repos/${REPO}/releases/latest"
readonly DOWNLOAD_BASE="https://github.com/${REPO}/releases/download"
readonly INSTALL_DIR="${HOME}/.local/bin"
readonly MARKER_START="# >>> reportal shell integration (do not edit) >>>"
readonly MARKER_END="# <<< reportal shell integration <<<"

err() {
    printf 'reportal-installer: error: %s\n' "$*" >&2
    exit 1
}

info() {
    printf 'reportal-installer: %s\n' "$*"
}

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "required command not found: $1"
    fi
}

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

# Detect target triple matching cargo-dist's build matrix.
# Supported targets (must match dist-workspace.toml):
#   aarch64-apple-darwin
#   x86_64-apple-darwin
#   aarch64-unknown-linux-gnu
#   x86_64-unknown-linux-gnu
detect_target() {
    local os arch
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64) echo "aarch64-apple-darwin" ;;
                x86_64)        echo "x86_64-apple-darwin" ;;
                *) err "unsupported macOS architecture: $arch" ;;
            esac
            ;;
        Linux)
            case "$arch" in
                aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
                x86_64|amd64)  echo "x86_64-unknown-linux-gnu" ;;
                *) err "unsupported Linux architecture: $arch" ;;
            esac
            ;;
        *)
            err "unsupported OS: $os (only macOS and Linux are supported by this installer; use cargo install reportal instead)"
            ;;
    esac
}

# Fetch a URL to stdout using whichever downloader is available.
fetch_stdout() {
    local url="$1"
    if have_cmd curl; then
        curl --proto '=https' --tlsv1.2 -fsSL "$url"
    elif have_cmd wget; then
        wget -qO- "$url"
    else
        err "need curl or wget to download from $url"
    fi
}

# Fetch a URL to a local path.
fetch_to_file() {
    local url="$1"
    local out="$2"
    if have_cmd curl; then
        curl --proto '=https' --tlsv1.2 -fsSL -o "$out" "$url"
    elif have_cmd wget; then
        wget -qO "$out" "$url"
    else
        err "need curl or wget to download from $url"
    fi
}

# Resolve release tag (either $REPORTAL_VERSION or latest from GitHub API).
resolve_tag() {
    if [ -n "${REPORTAL_VERSION:-}" ]; then
        echo "$REPORTAL_VERSION"
        return
    fi
    local body
    body=$(fetch_stdout "$API_LATEST") || err "failed to query $API_LATEST"
    # Extract "tag_name": "v0.15.0" without depending on jq.
    local tag
    tag=$(printf '%s\n' "$body" \
        | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | head -n1 \
        | sed 's/.*"\([^"]*\)"[[:space:]]*$/\1/')
    if [ -z "$tag" ]; then
        err "could not parse tag_name from $API_LATEST"
    fi
    echo "$tag"
}

# Verify SHA256 of $1 against the value in $2 (which may be the raw digest or
# "<digest>  <filename>" format). Prefers sha256sum, falls back to shasum.
verify_sha256() {
    local file="$1"
    local expected_file="$2"
    local expected
    expected=$(awk '{print $1; exit}' "$expected_file")
    if [ -z "$expected" ]; then
        err "empty checksum in $expected_file"
    fi

    local actual
    if have_cmd sha256sum; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif have_cmd shasum; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        info "warning: no sha256sum/shasum available; skipping checksum verification"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        err "checksum mismatch: expected $expected, got $actual"
    fi
    info "checksum OK"
}

# Determine shell rc file to modify.
detect_rc_file() {
    local shell_name
    shell_name=$(basename "${SHELL:-}")
    case "$shell_name" in
        zsh)  echo "zsh:${HOME}/.zshrc" ;;
        bash) echo "bash:${HOME}/.bashrc" ;;
        *)    echo "bash:${HOME}/.profile" ;;
    esac
}

# Append integration block to rc file, idempotently.
install_shell_integration() {
    local shell_name="$1"
    local rc_file="$2"

    if [ -f "$rc_file" ] && grep -Fq "$MARKER_START" "$rc_file"; then
        info "shell integration already present in $rc_file"
        return 0
    fi

    mkdir -p "$(dirname "$rc_file")"
    # Ensure trailing newline before appending.
    if [ -f "$rc_file" ] && [ -s "$rc_file" ]; then
        # Add a blank line separator if file does not already end with newline.
        if [ "$(tail -c1 "$rc_file" | wc -l)" -eq 0 ]; then
            printf '\n' >> "$rc_file"
        fi
    fi

    {
        printf '\n%s\n' "$MARKER_START"
        printf 'eval "$(rep init %s)"\n' "$shell_name"
        printf '%s\n' "$MARKER_END"
    } >> "$rc_file"

    info "appended shell integration to $rc_file"
}

# Print PATH warning if INSTALL_DIR isn't on PATH.
warn_path() {
    case ":${PATH:-}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            info "note: ${INSTALL_DIR} is not on your PATH."
            info "      add this to your shell rc file:"
            info "          export PATH=\"\$HOME/.local/bin:\$PATH\""
            ;;
    esac
}

main() {
    need_cmd uname
    need_cmd mktemp
    need_cmd tar
    need_cmd chmod
    need_cmd mkdir
    need_cmd grep
    need_cmd sed
    need_cmd awk

    local target tag archive url sha_url tmpdir archive_path sha_path
    target=$(detect_target)
    tag=$(resolve_tag)
    archive="reportal-${target}.tar.xz"
    url="${DOWNLOAD_BASE}/${tag}/${archive}"
    sha_url="${url}.sha256"

    info "installing reportal ${tag} for ${target}"

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    archive_path="${tmpdir}/${archive}"
    sha_path="${archive_path}.sha256"

    info "downloading ${url}"
    fetch_to_file "$url" "$archive_path" || err "failed to download $url"

    # Checksum file is optional (older releases may not have it).
    if fetch_to_file "$sha_url" "$sha_path" 2>/dev/null; then
        verify_sha256 "$archive_path" "$sha_path"
    else
        info "note: no .sha256 sidecar found; skipping checksum verification"
    fi

    info "extracting archive"
    tar -xf "$archive_path" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"

    # cargo-dist archives extract to a directory like reportal-<target>/.
    # Locate the binaries inside that directory.
    local extracted_dir
    extracted_dir="${tmpdir}/reportal-${target}"
    if [ ! -d "$extracted_dir" ]; then
        # Fall back to finding any extracted directory with a reportal binary.
        extracted_dir=$(find "$tmpdir" -mindepth 1 -maxdepth 2 -type f -name reportal -exec dirname {} \; | head -n1)
        if [ -z "$extracted_dir" ]; then
            err "could not locate reportal binary inside archive"
        fi
    fi

    local bin
    for bin in reportal rep; do
        if [ ! -f "${extracted_dir}/${bin}" ]; then
            err "missing binary in archive: ${bin}"
        fi
        install -m 0755 "${extracted_dir}/${bin}" "${INSTALL_DIR}/${bin}"
    done

    info "installed binaries to ${INSTALL_DIR}"
    warn_path

    # Shell integration.
    local rc_info shell_name rc_file
    rc_info=$(detect_rc_file)
    shell_name="${rc_info%%:*}"
    rc_file="${rc_info#*:}"
    install_shell_integration "$shell_name" "$rc_file"

    printf 'RePortal %s installed. Run '\''source %s'\'' or open a new shell.\n' \
        "$tag" "$rc_file"
}

main "$@"
