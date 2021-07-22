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
	docker run -p 12345:12345 -it $(server-tag)
