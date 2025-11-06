# Knowledge Graph

<img width="1385" height="1199" alt="image" src="https://github.com/user-attachments/assets/c386b181-b8d1-489f-bea3-d2ba3ded7754" />

# MacBook setup
(assuming you already have homebrew)

Install Rust
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

Install pnpm
```
brew install pnpm
```

Install developer tools
```
# Install cargo-watch for Rust hot-reloading
brew install cargo-watch

# Install just (task runner)
brew install just
```

Install UI dependencies
```
cd webui
pnpm install
cd ..
```

Start development
```
just dev
```