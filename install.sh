#!/bin/sh

# tmux-sessions install script
# Usage: curl -fsSL https://raw.githubusercontent.com/naicoi92/tmux-sessions/main/install.sh | sh

set -eu

# Colors for output - use printf for portability
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
REPO="naicoi92/tmux-sessions"
BINARY_NAME="tmux-sessions"
ALIAS_NAME="ts"

# Default values
VERSION="latest"
DRY_RUN=false

# Check if directory is writable
is_writable() {
    [ -w "$1" ] 2>/dev/null || [ -w "$(dirname "$1")" ] 2>/dev/null
}

# Get default install directory based on OS with auto-fallback
get_default_install_dir() {
    _os=$(detect_os)

    case "$_os" in
        linux)
            # Prefer /usr/local/bin if writable, otherwise fallback to ~/.local/bin
            if is_writable "/usr/local/bin"; then
                printf "/usr/local/bin"
            else
                printf "${HOME}/.local/bin"
            fi
            ;;
        darwin)
            printf "${HOME}/.local/bin"
            ;;
        *)
            printf "${HOME}/.local/bin"
            ;;
    esac
}

INSTALL_DIR=$(get_default_install_dir)

# Print functions using printf for POSIX compatibility
print_info() {
    printf "${BLUE}%s${NC} %s\n" "ℹ" "$1"
}

print_success() {
    printf "${GREEN}%s${NC} %s\n" "✓" "$1"
}

print_warning() {
    printf "${YELLOW}%s${NC} %s\n" "⚠" "$1"
}

print_error() {
    printf "${RED}%s${NC} %s\n" "✗" "$1"
}

# Detect OS
detect_os() {
    _os=$(uname -s | tr '[:upper:]' '[:lower:]')
    
    case "$_os" in
        linux)
            printf "linux"
            ;;
        darwin)
            printf "darwin"
            ;;
        *)
            print_error "Unsupported operating system: $_os"
            print_info "Supported: Linux, macOS"
            exit 1
            ;;
    esac
}

# Detect architecture (Rust target triple format)
detect_arch() {
    _arch=$(uname -m)

    case "$_arch" in
        x86_64|amd64)
            printf "x86_64"
            ;;
        aarch64|arm64)
            printf "aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $_arch"
            print_info "Supported: x86_64, aarch64/arm64"
            exit 1
            ;;
    esac
}

# Get Rust target triple suffix based on OS and arch
get_target_triple() {
    _os="$1"
    _arch="$2"

    case "${_os}-${_arch}" in
        darwin-x86_64)
            printf "x86_64-apple-darwin"
            ;;
        darwin-aarch64)
            printf "aarch64-apple-darwin"
            ;;
        linux-x86_64)
            printf "x86_64-unknown-linux-gnu"
            ;;
        linux-aarch64)
            printf "aarch64-unknown-linux-gnu"
            ;;
        *)
            print_error "Unsupported platform: ${_os}/${_arch}"
            exit 1
            ;;
    esac
}

# Get the latest release version
get_latest_version() {
    _version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name":' | \
        sed -e 's/.*"tag_name": "//' -e 's/".*//')
    
    if [ -z "$_version" ]; then
        print_error "Failed to get latest version"
        exit 1
    fi
    
    printf "%s" "$_version"
}

# Download URL construction
# Filename format: tmux-sessions-{arch}-{vendor}-{os} (Rust target triple format)
get_download_url() {
    _version="$1"
    _os="$2"
    _arch="$3"

    _target=$(get_target_triple "$_os" "$_arch")
    _filename="${BINARY_NAME}-${_target}"

    printf "https://github.com/%s/releases/download/%s/%s" "$REPO" "$_version" "$_filename"
}

# Main install function
install_tmux_sessions() {
    print_info "Installing tmux-sessions..."
    print_info "Repository: ${REPO}"
    
    # Detect platform
    _os=$(detect_os)
    _arch=$(detect_arch)
    
    print_info "Detected: ${_os}/${_arch}"
    
    # Get version
    if [ "$VERSION" = "latest" ]; then
        print_info "Fetching latest version..."
        VERSION=$(get_latest_version)
    fi
    
    print_info "Version: ${VERSION}"
    
    # Create install directory
    if [ "$DRY_RUN" = false ]; then
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Download URL
    _download_url=$(get_download_url "$VERSION" "$_os" "$_arch")
    
    print_info "Download URL: ${_download_url}"
    
    # Temporary directory
    _tmp_dir=$(mktemp -d)
    trap 'rm -rf "$_tmp_dir"' EXIT
    
    # Download
    _binary_file="${_tmp_dir}/${BINARY_NAME}"
    print_info "Downloading..."
    
    if [ "$DRY_RUN" = true ]; then
        print_info "[DRY RUN] Would download to: ${_binary_file}"
    else
        if ! curl -fsSL "$_download_url" -o "$_binary_file"; then
            print_error "Download failed!"
            print_info "The release may not exist yet for your platform."
            print_info "Supported platforms: linux-amd64, linux-arm64, darwin-amd64, darwin-arm64"
            exit 1
        fi
        print_success "Downloaded successfully"
    fi

    # Install binary
    _binary_path="${_binary_file}"
    _install_path="${INSTALL_DIR}/${BINARY_NAME}"
    
    print_info "Installing binary..."
    if [ "$DRY_RUN" = true ]; then
        print_info "[DRY RUN] Would install: ${_binary_path} → ${_install_path}"
    else
        if [ -f "$_install_path" ]; then
            print_warning "Existing installation found at ${_install_path}"
            rm -f "$_install_path"
        fi
        
        cp "$_binary_path" "$_install_path"
        chmod +x "$_install_path"
        print_success "Installed: ${_install_path}"
    fi
    
    # Create alias (ts)
    _alias_path="${INSTALL_DIR}/${ALIAS_NAME}"
    print_info "Creating alias '${ALIAS_NAME}'..."
    
    if [ "$DRY_RUN" = true ]; then
        print_info "[DRY RUN] Would create symlink: ${_alias_path} → ${_install_path}"
    else
        if [ -L "$_alias_path" ]; then
            rm -f "$_alias_path"
        fi
        
        ln -sf "$_install_path" "$_alias_path"
        print_success "Created alias: ${_alias_path}"
    fi
    
    # Check if install_dir is in PATH (only relevant for user directories)
    _path_check=$(printf ":%s:" "$PATH" | grep ":${INSTALL_DIR}:")
    if [ -z "$_path_check" ]; then
        print_warning "${INSTALL_DIR} is not in your PATH"
        print_info "Add the following to your shell configuration:"
        printf "\n"
        printf "  export PATH=\"${INSTALL_DIR}:\$PATH\"\n"
        printf "\n"
    fi

    # Success message
    printf "\n"
    print_success "Installation complete!"
    printf "\n"
    printf "  Binary: %s\n" "$_install_path"
    printf "  Alias:  %s\n" "$_alias_path"
    printf "\n"
    print_info "Next steps:"
    printf "\n"

    # Only show PATH export instruction when needed (user directories not in PATH)
    if [ -z "$_path_check" ] || [ "$_os" = "darwin" ]; then
        printf "  1. Ensure %s is in your PATH:\n" "${INSTALL_DIR}"
        printf "     export PATH=\"${INSTALL_DIR}:\$PATH\"\n"
        printf "\n"
        printf "  2. Add to your tmux.conf:\n"
    else
        printf "  1. Add to your tmux.conf:\n"
    fi
    printf "     bind-key -n M-w display-popup -h 80%% -w 80%% -E \"ts\"\n"
    printf "\n"

    if [ -z "$_path_check" ] || [ "$_os" = "darwin" ]; then
        printf "  3. Reload tmux:\n"
    else
        printf "  2. Reload tmux:\n"
    fi
    printf "     tmux source-file ~/.tmux.conf\n"
    printf "\n"

    if [ -z "$_path_check" ] || [ "$_os" = "darwin" ]; then
        printf "  4. Press Alt+w to launch!\n"
    else
        printf "  3. Press Alt+w to launch!\n"
    fi
    printf "\n"
}

# Show help
show_help() {
    cat << 'EOF'
tmux-sessions installer

USAGE:
    curl -fsSL https://raw.githubusercontent.com/naicoi92/tmux-sessions/main/install.sh | sh
    
    or with options:
    
    sh install.sh [OPTIONS]

OPTIONS:
    -v, --version VERSION    Install specific version (default: latest)
    -p, --prefix DIR         Install directory (default: ~/.local/bin)
    -d, --dry-run            Show what would be done without installing
    -h, --help               Show this help message

EXAMPLES:
    # Install latest
    curl -fsSL https://raw.githubusercontent.com/naicoi92/tmux-sessions/main/install.sh | sh
    
    # Install specific version
    sh install.sh --version v0.9.0
    
    # Install to custom directory
    sh install.sh --prefix ~/bin
    
    # Dry run
    sh install.sh --dry-run

EOF
}

# Parse arguments
parse_args() {
    while [ $# -gt 0 ]; do
        case $1 in
            -v|--version)
                VERSION="$2"
                shift 2
                ;;
            -p|--prefix)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Main entry point
main() {
    # Parse arguments
    parse_args "$@"
    
    # Check dependencies
    if ! command -v curl > /dev/null 2>&1; then
        print_error "curl is required but not installed"
        exit 1
    fi
    
    # Run installation
    install_tmux_sessions
}

main "$@"
