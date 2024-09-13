.PHONY: develop
develop:
	poetry install --no-root -vvv
	poetry run maturin develop -m acceptor/Cargo.toml
	poetry run maturin develop -m donor/Cargo.toml

.PHONY: test
test:	develop
	poetry run python -c "from acceptor import accept; from donor import generate; accept(generate())"
