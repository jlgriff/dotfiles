eval "$(starship init zsh)"

# Complete commands and paths with Tab.
autoload -Uz compinit
compinit
bindkey '^I' expand-or-complete

# Search history by the command prefix already typed.
autoload -Uz up-line-or-beginning-search down-line-or-beginning-search
zle -N up-line-or-beginning-search
zle -N down-line-or-beginning-search
bindkey '^[[A' up-line-or-beginning-search
bindkey '^[[B' down-line-or-beginning-search
bindkey '^[OA' up-line-or-beginning-search
bindkey '^[OB' down-line-or-beginning-search

# fnm
FNM_PATH="/opt/homebrew/opt/fnm/bin"
if [ -d "$FNM_PATH" ]; then
  eval "$(fnm env --shell zsh)"
fi
export PATH="$HOME/.local/bin:$PATH"
export PATH="$(brew --prefix rustup)/bin:$PATH"
