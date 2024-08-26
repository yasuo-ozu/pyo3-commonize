.PHONY: develop
develop:
	poetry install --no-root
	poetry run maturin develop -m acceptor/Cargo.toml
	poetry run maturin develop -m donor/Cargo.toml
