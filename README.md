# OdooPod

OdooPod is a Rust-based CLI tool to create and manage local Odoo instances without Docker.

The goal is to simplify Odoo development environment setup with:
- Odoo version management (16.0 to 19.0)
- automatic selection of compatible Python and PostgreSQL versions
- per-instance Python dependency isolation
- instance lifecycle orchestration (create, start, stop)

## Features

- fast and lightweight CLI
- automatic detection of available Odoo and longpolling ports
- persistent instance configuration
- local component management (Python/uv, PostgreSQL, Odoo sources)

## Installation (Unix)

To install OdooPod on Linux or macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/lucasmarcheguet/OdooPod/main/install.sh | bash
```

The installation script:
- automatically detects OS and architecture
- downloads the matching binary
- verifies integrity with SHA-256
- installs `odoopod` in `/usr/local/bin`

Verification:

```bash
odoopod --version
```

## Quick Start

Create an instance:

```bash
odoopod create my_instance 18.0
```

Start an instance:

```bash
odoopod start my_instance
```

List instances:

```bash
odoopod list
```

## Roadmap

- stabilize all lifecycle commands (`stop`, `remove`, `stop-all`)
- add Windows compatibility
- improve global and per-instance configuration
- add more observability commands (logs, shell, db-shell)