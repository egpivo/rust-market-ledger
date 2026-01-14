#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/color_map.sh"
source "${SCRIPT_DIR}/exit_code.sh"

RUSTUP_URL="https://sh.rustup.rs"
CARGO_HOME="${HOME}/.cargo"
RUSTUP_HOME="${HOME}/.rustup"

error() {
    echo -e "${FG_RED}Error:${FG_RESET} $1" >&2
    exit $ERROR_EXITCODE
}

success() {
    echo -e "${FG_GREEN}${1}${FG_RESET}"
}

info() {
    echo -e "${FG_BLUE}${1}${FG_RESET}"
}

warning() {
    echo -e "${FG_YELLOW}Warning:${FG_RESET} $1"
}

detect_os() {
    case "$(uname -s)" in
        Darwin*)
            echo "macos"
            ;;
        Linux*)
            echo "linux"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

check_cargo_installed() {
    if command -v cargo &> /dev/null; then
        info "Cargo is already installed: $(cargo --version)"
        return 0
    else
        return 1
    fi
}

install_rust_macos() {
    info "[macOS] Installing Rust via rustup..."
    
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed"
    fi
    
    if ! curl --proto '=https' --tlsv1.2 -sSf "$RUSTUP_URL" | sh -s -- -y --default-toolchain stable; then
        error "Failed to install Rust"
    fi
    
    if [ -f "${CARGO_HOME}/env" ]; then
        source "${CARGO_HOME}/env"
    fi
}

install_rust_linux() {
    info "[Linux] Installing Rust via rustup..."
    
    if ! command -v curl &> /dev/null; then
        warning "curl is required but not installed"
        info "Attempting to install curl..."
        
        if command -v apt-get &> /dev/null; then
            if ! sudo apt-get update && sudo apt-get install -y curl; then
                error "Failed to install curl via apt-get"
            fi
        elif command -v yum &> /dev/null; then
            if ! sudo yum install -y curl; then
                error "Failed to install curl via yum"
            fi
        elif command -v dnf &> /dev/null; then
            if ! sudo dnf install -y curl; then
                error "Failed to install curl via dnf"
            fi
        else
            error "Please install curl manually"
        fi
    fi
    
    if ! curl --proto '=https' --tlsv1.2 -sSf "$RUSTUP_URL" | sh -s -- -y --default-toolchain stable; then
        error "Failed to install Rust"
    fi
    
    if [ -f "${CARGO_HOME}/env" ]; then
        source "${CARGO_HOME}/env"
    fi
}

install_rust() {
    local os=$(detect_os)
    
    info "Detected OS: $os"
    
    case "$os" in
        macos)
            install_rust_macos
            ;;
        linux)
            install_rust_linux
            ;;
        *)
            error "Unsupported operating system: $(uname -s). Please install Rust manually from: https://www.rust-lang.org/tools/install"
            ;;
    esac
}

main() {
    echo -e "${FG_BLUE}Rust & Cargo Installation Script${FG_RESET}"
    echo "================================="
    echo ""
    
    if check_cargo_installed; then
        echo ""
        success "Rust is already installed. No action needed."
        info "To update Rust, run: rustup update"
        exit $SUCCESS_EXITCODE
    fi
    
    info "Rust/Cargo not found. Starting installation..."
    echo ""
    
    install_rust
    
    echo ""
    info "Verifying installation..."
    
    if [ -f "${CARGO_HOME}/env" ]; then
        source "${CARGO_HOME}/env"
    fi
    
    if command -v cargo &> /dev/null; then
        echo ""
        success "Installation successful!"
        echo ""
        info "Cargo version: $(cargo --version)"
        info "Rust version: $(rustc --version)"
        echo ""
        warning "If cargo is not available in your current shell, run:"
        echo "  source ${CARGO_HOME}/env"
        echo ""
        info "Or add this to your ~/.bashrc or ~/.zshrc:"
        echo "  source ${CARGO_HOME}/env"
        exit $SUCCESS_EXITCODE
    else
        echo ""
        error "Installation may have failed. Please try manually:\n  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
}

main "$@"
