# Nmap GUI Wrapper

A **simple, cross-platform GUI wrapper** for `nmap` written in **Rust** using **egui** (`eframe`).  
Scans your local `/24` network from the **default gateway**, displays live hosts with **hostnames**, and includes **dark mode**, **copy-to-clipboard**, and **raw output**.

---

## Features

- **Auto-detects default gateway** on startup (Windows, Linux, macOS)
- **One-click scan** of `gateway/24` (e.g. `10.0.0.1/24`)
- **Live host list** with:
  - IP address
  - **Hostname** (via reverse DNS with `-R`)
- **Copy all results** to clipboard
- **Dark / Light mode toggle** (persisted)
- **Raw nmap output** (collapsible, for debugging)
- **No admin/root required** (uses `-sn` ping scan)

---

## Screenshot

*(Coming soon — or run it yourself!)*

---

## Requirements

| Tool | Minimum Version |
|------|-----------------|
| [Rust](https://rustup.rs) | 1.70+ |
| [nmap](https://nmap.org/download.html) | 7.0+ |
| OS | Windows, Linux, macOS |

> `nmap` must be in your `PATH`.

---

## Installation

### 1. Clone & Build

```bash
git clone https://github.com/yourname/nmap-gui.git
cd nmap-gui
cargo build --release
```

> Binary will be in `target/release/nmap-gui(.exe)`

### 2. Install `nmap`

#### Windows
- Download from [nmap.org](https://nmap.org/download.html)
- Run installer → **Add to PATH**

#### Linux
```bash
sudo apt install nmap    # Ubuntu/Debian
sudo dnf install nmap    # Fedora
```

#### macOS
```bash
brew install nmap
```

---

## Usage

```bash
./target/release/nmap-gui
```

Or double-click the executable.

### UI Guide

1. **App starts** → shows your default gateway (e.g. `10.0.0.1`)
2. Click **"Scan /24"**
3. Wait ~5–15 seconds
4. **Live hosts appear** with hostnames
5. Click **"Copy all"** to copy results
6. Toggle **Dark mode** (top-right)

---

## How It Works

- **Gateway detection**:
  - Windows: `netsh interface ip show address`
  - Linux/macOS: `ip route show default`
- **Scan command**:
  ```bash
  nmap -sn -R --dns-servers 1.1.1.1 --host-timeout 5s -oX - 10.0.0.1/24
  ```
- **XML parsing** via `serde-xml-rs`
- **Background thread** → UI stays responsive
- **Robust parsing** handles:
  - Multiple `<address>` tags
  - Missing/empty `<hostnames/>`
  - IPv4 + MAC filtering

---

## Project Structure

```
nmap-gui/
├── Cargo.toml
├── src/
│   └── main.rs          # All logic (GUI + nmap + parsing)
├── README.md
└── target/              # Built binary
```

---

## Development

```bash
cargo run --release     # Run with hot-reload (debug)
cargo run --release     # Final optimized build
```

### Dependencies

```toml
eframe = "0.28"
serde = { version = "1.0", features = ["derive"] }
serde-xml-rs = "0.6"
```

---

## Legal & Safety

- **Only scans your local network** (`/24` from gateway)
- **Ping scan only** (`-sn`) — no port scanning
- **Legal on networks you own or have permission to scan**
- **Never use on public or unauthorized networks**

---

## Roadmap

| Feature | Status |
|-------|--------|
| Auto-scan on startup | Not started |
| Refresh button | Not started |
| Export to CSV/JSON | Not started |
| Show MAC address | Not started |
| System tray + minimize | Not started |
| Port scan mode | Not started |

---

## Contributing

1. Fork it
2. Create your feature branch (`git checkout -b feature/xyz`)
3. Commit (`git commit -m 'Add xyz'`)
4. Push (`git push origin feature/xyz`)
5. Open a Pull Request

---

## License

```
MIT License
```

See [LICENSE](LICENSE) for details.

---

## Author

Brandon S Smith

---

> **Enjoy your fast, beautiful network scanner!**
