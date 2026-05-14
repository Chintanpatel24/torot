#compdef torot

_torot_commands() {
    local -a commands
    commands=(
        'tools:List detected security tools'
        'scan:Run a security scan against a target'
        'report:Generate a report for a session'
        'config:Show current configuration'
        'help:Show help information'
    )
    _describe 'command' commands
}

_torot_scan() {
    local -a args
    args=(
        '--target[Target URL, host, or path]'
        '(-t)'{--target}'[Target URL, host, or path]'
        '--tools[Comma-separated tool list]'
        '(-m)'{--mode}'[Scan mode]:mode:((single\:"One pass, all tools" deep\:"Multi-wave exhaustive" passive\:"No active requests"))'
        '(-o)'{--output}'[Report output path]'
        '--template-file[Custom report template file]'
    )
    _arguments $args
}

_torot_report() {
    local -a args
    args=(
        '(-s)'{--session}'[Session ID]'
        '(-o)'{--output}'[Report output path]'
        '--template-file[Custom report template file]'
    )
    _arguments $args
}

_torot() {
    if ((CURRENT == 2)); then
        _torot_commands
    else
        local cmd=${words[2]}
        case $cmd in
            scan)   _torot_scan ;;
            report) _torot_report ;;
            *)      _torot_commands ;;
        esac
    fi
}

compdef _torot torot
