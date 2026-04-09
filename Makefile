.PHONY: help install install-linux-deps dev prod prod-build prod-run dev-gateway build test lint format clean \
        format-all lint-all test-all

FRONTEND_BUILD_OUT := tauri-app/dist/index.html
FRONTEND_BUILD_INPUTS := $(shell git ls-files tauri-app/src) tauri-app/index.html tauri-app/vite.config.ts tauri-app/package.json tauri-app/package-lock.json tauri-app/.env.production

RUST_RELEASE_BIN := tauri-app/src-tauri/target/release/tauri-app
RUST_RELEASE_INPUTS := $(shell git ls-files tauri-app/src-tauri/src crdt-core/src) tauri-app/src-tauri/Cargo.toml tauri-app/src-tauri/Cargo.lock crdt-core/Cargo.toml

install-linux-deps:
	sudo apt-get update
	sudo apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# install frontend dependencies
install:
	cd tauri-app && npm install


# ------------- formatting -------------- 
format-crdt:
	cd crdt-core && cargo fmt

format-tauri:
	cd tauri-app/src-tauri && cargo fmt

format-go:
	cd gateway && go fmt ./...

format-frontend:
	cd tauri-app && npx prettier --write "src/**/*.{ts,tsx,css}"

format-all: format-crdt format-tauri format-go format-frontend


# -------------linting ---------------
lint-frontend:
	cd tauri-app && npm run lint

lint-crdt:
	cd crdt-core && cargo clippy --all-targets --all-features -- -D warnings

lint-tauri:
	cd tauri-app/src-tauri && cargo clippy --all-targets --all-features

lint-go:
	cd gateway && go vet ./...

lint-all: lint-frontend lint-crdt lint-tauri lint-go


# ------------- testing ---------------
test-crdt:
	cd crdt-core && cargo test

test-tauri:
	cd tauri-app/src-tauri && cargo test

test-go:
	cd gateway && go test -v ./...

test-all: test-crdt test-tauri test-go


# ------------- development ---------------

# run development servers for Tauri app
dev:
	cd tauri-app && npm run tauri:dev

$(FRONTEND_BUILD_OUT): $(FRONTEND_BUILD_INPUTS)
	cd tauri-app && npm run build

$(RUST_RELEASE_BIN): $(RUST_RELEASE_INPUTS)
	cd tauri-app && npm run tauri build -- --no-bundle

prod-build: $(FRONTEND_BUILD_OUT) $(RUST_RELEASE_BIN)

prod-run: $(RUST_RELEASE_BIN)
	cd tauri-app/src-tauri && ./target/release/tauri-app

# run production build/runtime for Tauri app (rebuilds only when needed)
prod: prod-build prod-run

# run development server for Go Gateway
dev-gateway:
	cd gateway && go run main.go

# build whole app
build:
	cd gateway && go build -o bin/gateway main.go
	cd tauri-app && npm run build
	cd tauri-app && npm run tauri build

# clean up build artifacts
clean:
	rm -rf tauri-app/node_modules
	rm -rf tauri-app/dist
	rm -rf tauri-app/src-tauri/target
	rm -rf crdt-core/target
	rm -rf gateway/bin
