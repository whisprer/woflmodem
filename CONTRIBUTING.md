[CONTRIBUTING.md]

# Contributing to woflmodem

Thanks for your interest in improving woflmodem HSF softmodem!


## Code Style

- Follow **Rust 2021** edition conventions.
- Run `cargo fmt` before committing.
- Ensure all warnings are resolved: `cargo clippy -- -D warnings`
- Minimize dependencies - prefer standard library solutions where possible.
- Write idiomatic Rust: use iterators, pattern matching, and zero-cost abstractions.


## Development Workflow

1. **Fork the repository** on GitHub.

2. **Clone your fork**:
git clone https://github.com/YOUR_USERNAME/woflmodem.git
cd welcomed

3. **Create a feature branch**:
git checkout -b feature/your-feature-name

4. **Make your changes** and commit with clear, conventional messages:
git commit -m "feat: add V.32 protocol support"
git commit -m "fix: correct phase offset in demodulator"
git commit -m "docs: update API documentation for audio module"

Use conventional commit prefixes:
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `test:` - Test additions or modifications
- `refactor:` - Code restructuring without behavior changes
- `perf:` - Performance improvements
- `security:` - Security-related changes

5. **Push to your fork**:
git push origin feature/your-feature-name

text

6. **Open a Pull Request** against `main` with a detailed description.


## Testing

All contributions must include appropriate tests.

### Run the Full Test Suite
cargo test


### Run Specific Tests
cargo test --test integration_tests
cargo test --lib modem::tests


### Run with Logging
RUST_LOG=debug cargo test


### Manual Testing
For DSP and audio changes, perform manual verification:
1. Run the modem with real audio I/O.
2. Verify signal quality with an oscilloscope or audio analyzer.
3. Test on multiple platforms (Windows/macOS/Linux).
4. Verify compatibility with physical modems if possible.


## Documentation

- Update `README.md` for user-facing features.
- Add inline documentation (`///`) for public APIs.
- Update `CHANGELOG.md` following existing format.
- Include examples in doc comments:
/// Modulates a data stream using V.22 FSK.
///
/// # Example
/// /// let data = vec![0x01, 0x02, 0x03]; /// let modulated = v22::modulate(&data)?; ///
pub fn modulate(data: &[u8]) -> Result<Vec<f32>> { ... }