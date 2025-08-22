#!/usr/bin/env bash
#
# VModem Model 99/A - Virtual Modem Terminal
# A nostalgic modem simulator with modern connectivity
# Requires: minimodem, curl, wget, figlet, argc.rs
#

# Color codes for retro terminal aesthetics
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly MAGENTA='\033[0;35m'
readonly CYAN='\033[0;36m'
readonly WHITE='\033[1;37m'
readonly BOLD='\033[1m'
readonly DIM='\033[2m'
readonly BLINK='\033[5m'
readonly NC='\033[0m' # No Color

# Configuration
MODEM_SPEED=1200
CONNECTION_TYPE="hayes"
LOG_FILE="$HOME/.vmodem99a.log"
CONFIG_FILE="$HOME/.vmodem99a.conf"

# Load configuration if exists
[[ -f "$CONFIG_FILE" ]] && source "$CONFIG_FILE"

# Logging function
log_action() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$LOG_FILE"
}

# Sound effects using minimodem
play_dial_tone() {
    echo -e "${CYAN}♪ Dialing...${NC}"
    echo "ATDT" | minimodem --tx -a $MODEM_SPEED &
    SOUND_PID=$!
    sleep 2
    kill $SOUND_PID 2>/dev/null
}

play_handshake() {
    echo -e "${YELLOW}♪ Handshaking...${NC}"
    echo "CONNECT $MODEM_SPEED" | minimodem --tx -a $MODEM_SPEED &
    SOUND_PID=$!
    sleep 1
    kill $SOUND_PID 2>/dev/null
}

play_disconnect() {
    echo -e "${RED}♪ Disconnecting...${NC}"
    echo "+++ATH" | minimodem --tx -a $MODEM_SPEED &
    SOUND_PID=$!
    sleep 1
    kill $SOUND_PID 2>/dev/null
}

# Banner display
show_banner() {
    clear
    echo -e "${CYAN}"
    figlet -f small "VModem 99/A" 2>/dev/null || echo "VModem Model 99/A"
    echo -e "${DIM}═══════════════════════════════════════════════════════${NC}"
    echo -e "${MAGENTA}Virtual Modem Terminal v1.0 - Hayes Compatible${NC}"
    echo -e "${DIM}Baud Rate: ${YELLOW}$MODEM_SPEED${DIM} | Protocol: ${YELLOW}$CONNECTION_TYPE${NC}"
    echo -e "${DIM}═══════════════════════════════════════════════════════${NC}"
    echo
}

# Status display
show_status() {
    echo -e "${BLUE}[STATUS]${NC} $1"
    log_action "STATUS: $1"
}

# Error display
show_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    log_action "ERROR: $1"
}

# Success display
show_success() {
    echo -e "${GREEN}[OK]${NC} $1"
    log_action "SUCCESS: $1"
}

# HTTP connection via curl
connect_http() {
    local url="$1"
    local method="${2:-GET}"
    
    show_status "Initializing HTTP connection to $url"
    play_dial_tone
    
    echo -e "${YELLOW}Connecting via HTTP...${NC}"
    
    case "$method" in
        "GET")
            if curl -s --connect-timeout 10 -w "HTTP %{http_code} | Size: %{size_download} bytes | Time: %{time_total}s\n" "$url"; then
                play_handshake
                show_success "HTTP GET connection established"
            else
                show_error "HTTP connection failed"
                return 1
            fi
            ;;
        "HEAD")
            if curl -sI --connect-timeout 10 "$url"; then
                play_handshake
                show_success "HTTP HEAD request completed"
            else
                show_error "HTTP HEAD request failed"
                return 1
            fi
            ;;
    esac
}

# Download via wget
download_file() {
    local url="$1"
    local output="${2:-$(basename "$url")}"
    
    show_status "Initiating file transfer from $url"
    play_dial_tone
    
    echo -e "${CYAN}Downloading via WGET protocol...${NC}"
    
    if wget --progress=bar --timeout=30 -O "$output" "$url" 2>&1; then
        play_handshake
        show_success "File downloaded successfully: $output"
    else
        show_error "Download failed"
        return 1
    fi
}

# SSH connection
connect_ssh() {
    local host="$1"
    local user="${2:-$USER}"
    local port="${3:-22}"
    
    show_status "Establishing SSH connection to $user@$host:$port"
    play_dial_tone
    
    echo -e "${GREEN}Connecting via SSH protocol...${NC}"
    
    if command -v ssh >/dev/null 2>&1; then
        play_handshake
        show_success "SSH handshake complete - opening terminal"
        ssh -p "$port" "$user@$host"
        play_disconnect
        show_status "SSH connection terminated"
    else
        show_error "SSH client not available"
        return 1
    fi
}

# Telnet connection
connect_telnet() {
    local host="$1"
    local port="${2:-23}"
    
    show_status "Establishing Telnet connection to $host:$port"
    play_dial_tone
    
    echo -e "${MAGENTA}Connecting via TELNET protocol...${NC}"
    
    if command -v telnet >/dev/null 2>&1; then
        play_handshake
        show_success "Telnet connection established"
        telnet "$host" "$port"
        play_disconnect
        show_status "Telnet connection terminated"
    else
        show_error "Telnet client not available"
        return 1
    fi
}

# Mosh connection (if available)
connect_mosh() {
    local host="$1"
    local user="${2:-$USER}"
    
    show_status "Establishing Mosh connection to $user@$host"
    play_dial_tone
    
    if command -v mosh >/dev/null 2>&1; then
        echo -e "${CYAN}Connecting via MOSH protocol...${NC}"
        play_handshake
        show_success "Mosh connection established"
        mosh "$user@$host"
        play_disconnect
        show_status "Mosh connection terminated"
    else
        show_error "Mosh client not available"
        return 1
    fi
}

# Configuration menu
configure_modem() {
    echo -e "${YELLOW}Modem Configuration${NC}"
    echo -e "${DIM}────────────────────${NC}"
    echo "1) Baud Rate (current: $MODEM_SPEED)"
    echo "2) Connection Type (current: $CONNECTION_TYPE)"
    echo "3) Reset to defaults"
    echo "4) Back to main menu"
    echo
    read -p "Select option: " choice
    
    case "$choice" in
        1)
            echo "Available baud rates: 300, 1200, 2400, 9600, 14400, 28800, 56000"
            read -p "Enter baud rate: " new_speed
            if [[ "$new_speed" =~ ^[0-9]+$ ]]; then
                MODEM_SPEED="$new_speed"
                echo "MODEM_SPEED=$MODEM_SPEED" > "$CONFIG_FILE"
                show_success "Baud rate set to $MODEM_SPEED"
            else
                show_error "Invalid baud rate"
            fi
            ;;
        2)
            echo "Available types: hayes, bell, v90, v92"
            read -p "Enter connection type: " new_type
            CONNECTION_TYPE="$new_type"
            echo "CONNECTION_TYPE=$CONNECTION_TYPE" >> "$CONFIG_FILE"
            show_success "Connection type set to $CONNECTION_TYPE"
            ;;
        3)
            rm -f "$CONFIG_FILE"
            MODEM_SPEED=1200
            CONNECTION_TYPE="hayes"
            show_success "Configuration reset to defaults"
            ;;
    esac
}

# Phone book functionality
show_phonebook() {
    echo -e "${CYAN}VModem Phone Book${NC}"
    echo -e "${DIM}─────────────────${NC}"
    echo "Recent connections:"
    if [[ -f "$LOG_FILE" ]]; then
        tail -10 "$LOG_FILE" | grep -E "(HTTP|SSH|TELNET|MOSH)" | sed 's/^/  /'
    else
        echo "  No recent connections"
    fi
    echo
}

# Help system
show_help() {
    echo -e "${GREEN}VModem Model 99/A Help${NC}"
    echo -e "${DIM}═════════════════════${NC}"
    echo
    echo -e "${BOLD}Available Commands:${NC}"
    echo -e "  ${CYAN}http <url> [method]${NC}     - Connect via HTTP (GET/HEAD)"
    echo -e "  ${CYAN}download <url> [file]${NC}   - Download file via wget"
    echo -e "  ${CYAN}ssh <host> [user] [port]${NC} - Connect via SSH"
    echo -e "  ${CYAN}telnet <host> [port]${NC}    - Connect via Telnet"
    echo -e "  ${CYAN}mosh <host> [user]${NC}      - Connect via Mosh"
    echo -e "  ${CYAN}config${NC}                  - Configure modem settings"
    echo -e "  ${CYAN}phonebook${NC}               - View connection history"
    echo -e "  ${CYAN}help${NC}                    - Show this help"
    echo -e "  ${CYAN}quit${NC}                    - Exit VModem"
    echo
    echo -e "${BOLD}Examples:${NC}"
    echo -e "  ${DIM}http https://httpbin.org/ip${NC}"
    echo -e "  ${DIM}download https://example.com/file.txt${NC}"
    echo -e "  ${DIM}ssh user@example.com${NC}"
    echo -e "  ${DIM}telnet towel.blinkenlights.nl${NC}"
    echo
}

# Main command dispatcher
handle_command() {
    local cmd="$1"
    shift
    
    case "$cmd" in
        "http")
            [[ -z "$1" ]] && { show_error "URL required"; return 1; }
            connect_http "$@"
            ;;
        "download"|"dl")
            [[ -z "$1" ]] && { show_error "URL required"; return 1; }
            download_file "$@"
            ;;
        "ssh")
            [[ -z "$1" ]] && { show_error "Host required"; return 1; }
            connect_ssh "$@"
            ;;
        "telnet")
            [[ -z "$1" ]] && { show_error "Host required"; return 1; }
            connect_telnet "$@"
            ;;
        "mosh")
            [[ -z "$1" ]] && { show_error "Host required"; return 1; }
            connect_mosh "$@"
            ;;
        "config"|"configure")
            configure_modem
            ;;
        "phonebook"|"pb")
            show_phonebook
            ;;
        "help"|"?")
            show_help
            ;;
        "clear"|"cls")
            show_banner
            ;;
        "quit"|"exit"|"bye")
            echo -e "${YELLOW}Hanging up modem...${NC}"
            play_disconnect
            echo -e "${GREEN}73! Thanks for using VModem 99/A${NC}"
            exit 0
            ;;
        "")
            # Empty command, do nothing
            ;;
        *)
            show_error "Unknown command: $cmd (type 'help' for commands)"
            ;;
    esac
}

# Interactive mode
interactive_mode() {
    show_banner
    echo -e "${GREEN}Ready! Type 'help' for commands or 'quit' to exit.${NC}"
    echo
    
    while true; do
        echo -ne "${BOLD}${CYAN}VModem>${NC} "
        read -r input
        [[ -z "$input" ]] && continue
        
        # Parse command and arguments
        read -ra ARGS <<< "$input"
        handle_command "${ARGS[@]}"
        echo
    done
}

# Command line argument handling (argc.rs style)
main() {
    # Check for required dependencies
    local missing_deps=()
    
    command -v minimodem >/dev/null || missing_deps+=("minimodem")
    command -v curl >/dev/null || missing_deps+=("curl")
    command -v wget >/dev/null || missing_deps+=("wget")
    command -v figlet >/dev/null || missing_deps+=("figlet")
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        echo -e "${RED}Missing dependencies: ${missing_deps[*]}${NC}"
        echo "Please install the missing packages and try again."
        exit 1
    fi
    
    # If no arguments, start interactive mode
    if [[ $# -eq 0 ]]; then
        interactive_mode
    else
        # Handle command line arguments
        show_banner
        handle_command "$@"
    fi
}

# Initialize log file
touch "$LOG_FILE"
log_action "VModem 99/A started"

# Run main function with all arguments
main "$@"
