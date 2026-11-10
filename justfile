set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

run *args:
    cargo run -p fern -- {{ args }}

fmt:
    cargo fmt --all

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo nextest run --workspace

deny:
    cargo deny check

shear:
    cargo shear

typos:
    typos

zizmor:
    zizmor .

check: lint test deny shear typos zizmor
