test-with-cloud:
	cargo test --features=test-using-sa

test-api:
	cargo test --features=test-using-sa api_

test-get-headers:
	cargo test --features=test-using-sa get_headers

test-get-values:
	cargo test --features=test-using-sa get_values

test-all:
	cargo test --all-features

run-local:
	cargo run -- -s=./dev-secret/test-sa-key.json

build-playground:
	cd playground && yarn export-prd && rm -rf ../src/playground_html && cp -r out ../src/playground_html

build-prd-restricted:
	cargo build --release --features=restricted


