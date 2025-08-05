# RustDress
Self hosted Lightning Address Server and NIP-05 Verification Server
<br></br>

### For Docker:

- Check the `Dockerfile` and the `sample-docker-compose.yaml` file.

### To build from source:
Pre-requisites:
You will need rust installed.
https://www.rust-lang.org/


```bash
#Clone the repo
git clone https://github.com/niteshbalusu11/rustdress.git

# Change directory
cd rustdress

# Copy the sample config file and edit it
cp sample_rustdress.toml rustdress.toml

# Run the app, passing the config file path
# Default path is ~/.config/rustdress/rustdress.toml
cargo run --release -- --config /path/to/rustdress.toml

# You might have to install cmake, pkg-config

sudo apt-get install cmake pkg-config
```

### Using nix

- You need (https://nixos.org/)[nix] installed
- Install (https://devenv.sh/getting-started/)[devenv]

```
devenv shell

# Then you can use cargo commands as usual
```

- Github Actions have also been updated to use nix and devenv to match local development.
