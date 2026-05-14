_torot() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    if [[ ${COMP_CWORD} == 1 ]]; then
        opts="tools scan report config help"
        COMPREPLY=($(compgen -W "${opts}" -- ${cur}))
        return 0
    fi

    case "${COMP_WORDS[1]}" in
        scan)
            case ${prev} in
                --target|-t)    return 0 ;;
                --tools)        return 0 ;;
                --mode|-m)      COMPREPLY=($(compgen -W "single deep passive" -- ${cur})); return 0 ;;
                --output|-o)    return 0 ;;
                --template-file) return 0 ;;
                *)              opts="--target --tools --mode --output --template-file" ;;
            esac
            ;;
        report)
            case ${prev} in
                --session|-s)   return 0 ;;
                --output|-o)    return 0 ;;
                --template-file) return 0 ;;
                *)              opts="--session --output --template-file" ;;
            esac
            ;;
    esac

    COMPREPLY=($(compgen -W "${opts}" -- ${cur}))
    return 0
}

complete -F _torot torot
