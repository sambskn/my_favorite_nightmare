itch_project := env('ITCH_IO_PROJECT_ID')

dev:
    cargo run

wasm-build:
    bevy build web --bundle

wasm-deploy:
    # assumes you have butler installed and logged in
    butler push target/bevy_web/web/my_favorite_nightmare "{{itch_project}}:wasm"

wasm-check:
    cargo c --target wasm32-unknown-unknown
