build database="sqlite":
    just docs
    just style
    bun i
    bun run static_build.ts
    cargo build -r --no-default-features --features {{database}}

docs:
    cargo doc --no-deps --document-private-items

test:
    just docs
    just style
    bun run static_build.ts
    cargo run

run:
    chmod +x ./target/release/guppy
    ./target/release/guppy

style:
    bunx tailwindcss -i ./static/input.css -o ./static/style.css
