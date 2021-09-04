server-tag = "passage-server"

.PHONY: build
build: build-prod

.PHONY: build-prod
build-prod:
	docker build -t $(server-tag) .

.PHONY: build-dev
build-dev:
	docker build -t $(server-tag) --target development .

.PHONY: run
run: build-dev
	docker run --rm -p 12345:12345 -it $(server-tag)

.PHONY: run-leader
run-leader: data
	cargo run --bin passage-server --\
		--cluster-nodes 127.0.0.1:12344 \
		--log-file "data/leader-wal.txt"

.PHONY: run-follower
run-follower-1: data
	cargo run --bin passage-server --\
		--port 12344 \
		--read-only \
		--log-file "data/follower-1-wal.txt"

data:
	mkdir data
