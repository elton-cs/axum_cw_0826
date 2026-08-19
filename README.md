# Crosswordle server

The Axum application serves both the `/game/*` JSON API and the statically built React frontend.

## Run locally

Build the frontend before starting the server:

```bash
cd ../tstack_cw_0826
pnpm install
pnpm build

cd ../axum_cw_0826
cargo run --release
```

Open `http://127.0.0.1:3000`. Client-side routes fall back to `dist/index.html`, so direct links and browser refreshes work.

## Configuration

- `ADDRESS`: listener address; defaults to `127.0.0.1:3000`. For a container or public host, use `0.0.0.0:3000`.
- `FRONTEND_DIST`: path to the Vite output directory. It defaults to the sibling `tstack_cw_0826/dist` directory and may be absolute or relative to the process working directory.

Nitro is no longer used. Node/pnpm are needed only to build the frontend assets, not at runtime.
