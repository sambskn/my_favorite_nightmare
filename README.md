# my_favorite_nightmare
Bevy Jam 7

## Deploying to itch.io
- WASM
  - run `just wasm-build wasm-deploy`
    - after running the first time, edit the itch project and mark this channel 'wasm' as being what gets run in the web
- Other
  - idk man probably just run `cargo build --release` then copy the binary along with the assets folder and bada bing! you got a game ready to be zipped up
