os := `grep -E '^(NAME)=' /etc/os-release`
run:
    RUST_LOG=DEBUG cargo r

tui:
    cargo r -- tui

tui-nix:
    RUST_LOG=DEBUG nix run . -- tui
