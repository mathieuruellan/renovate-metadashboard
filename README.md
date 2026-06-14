# renovate-metadashboard

A dashboard for Renovate dependency updates.

## Agent constraint

The project **must compile** at all times. Any code change must satisfy:

- `cargo build --release --target=x86_64-unknown-linux-musl`
- `docker build .` (full static build from `scratch`, no OpenSSL)
- The Docker image must produce a statically linked binary (no dynamic NEEDED entries)

## Building

> **Note:** The Rust build (especially the first run with dependency compilation) can take
> several minutes. There is no timeout issue — it's just slow. Be patient.

```bash
# Local build
cargo build --release --target=x86_64-unknown-linux-musl

# Docker build (also slow on first run)
docker build -t renovate-metadashboard .
```
