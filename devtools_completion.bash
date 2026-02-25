#!/bin/bash

_devtools() {
    local cur prev words cword
    _init_completion || return

    local commands="main-db image-db log-db fetch-db fetch-main-db fetch-image-db fetch-log-db deploy gen-main-schema install-completions"

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=($(compgen -W "$commands" -- "$cur"))
        return
    fi

    local cmd="${words[1]}"
    case "$cmd" in
        main-db|image-db|log-db)
            COMPREPLY=($(compgen -W "$(sqlite3 ~/.local/share/btcmap/btcmap.db '.tables' 2>/dev/null)" -- "$cur"))
            ;;
    esac
}

complete -F _devtools devtools
