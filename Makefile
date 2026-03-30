.PHONY: clean clean-target clean-fuzz clean-native clean-artifacts clean-r clean-docs clean-docs-deps clean-tool-cache distclean install-renjin compiler-parallel-smoke library-package-suite optimizer-suite-legality optimizer-suite-heavy optimizer-suite-all perf-gate recommended-package-coverage test-tier0 test-tier1 test-tier2-main test-tier2-differential

RENJIN_VERSION := 3.5-beta76
RENJIN_URL := https://www.renjin.org/repo/dist/renjin-$(RENJIN_VERSION).zip
TOOL_CACHE_DIRS := target/tmp/bench-python target/tmp/renjin-dist

clean: clean-target clean-fuzz clean-native clean-artifacts clean-r clean-docs
	@echo "[clean] done"

clean-target:
	@echo "[clean] removing Rust build artifacts (preserving tooling caches)"
	@stash=$$(mktemp -d); \
	for d in $(TOOL_CACHE_DIRS); do \
		if [ -d "$$d" ]; then \
			mkdir -p "$$stash/$$(dirname "$$d")"; \
			mv "$$d" "$$stash/$$d"; \
		fi; \
	done; \
	cargo clean; \
	for d in $(TOOL_CACHE_DIRS); do \
		if [ -d "$$stash/$$d" ]; then \
			mkdir -p "$$(dirname "$$d")"; \
			mv "$$stash/$$d" "$$d"; \
		fi; \
	done; \
	rm -rf "$$stash"

clean-fuzz:
	@echo "[clean] removing fuzz artifacts"
	@rm -rf fuzz/artifacts
	@rm -rf fuzz/coverage
	@rm -rf fuzz/target
	@rm -f fuzz/*.profraw fuzz/*.profdata fuzz/*.log

clean-native:
	@echo "[clean] removing native build outputs"
	@rm -f native/*.so native/*.dylib native/*.dll native/*.o native/*.a

clean-artifacts:
	@echo "[clean] removing generated reports, plots, csv outputs, and triage artifacts"
	@rm -rf .artifacts
	@rm -f Rplots.pdf
	@find . \
		\( \
			-path './docs/node_modules' -o \
			-path './docs/node_modules/*' -o \
			-path './target/tmp/bench-python' -o \
			-path './target/tmp/bench-python/*' -o \
			-path './target/tmp/renjin-dist' -o \
			-path './target/tmp/renjin-dist/*' \
		\) -prune -o \
		-type f \( \
		-name '*.png' -o \
		-name '*.pdf' -o \
		-name '*.jpg' -o \
		-name '*.jpeg' -o \
		-name '*.bmp' -o \
		-name '*.tiff' -o \
		-name 'rr_*.csv' -o \
		-name '*.Rout' -o \
		-name '.Rhistory' -o \
		-name '.RData' -o \
		-name '__triage_smoke_*' \
	\) -print0 | \
	while IFS= read -r -d '' f; do \
		if git ls-files --error-unmatch "$$f" >/dev/null 2>&1; then \
			continue; \
		fi; \
		rm -f "$$f"; \
	done
	@find tests -type d -name '__triage_smoke_*' -prune -exec rm -rf {} +

clean-r:
	@echo "[clean] removing untracked generated R files"
	@find . -type f \( -name '*.R' -o -name '*.r' -o -name '*.gen.R' \) -print0 | \
	while IFS= read -r -d '' f; do \
		if [ "$$f" = "./src/runtime/runtime_prelude.R" ] || [ "$$f" = "src/runtime/runtime_prelude.R" ]; then \
			continue; \
		fi; \
		if git ls-files --error-unmatch "$$f" >/dev/null 2>&1; then \
			continue; \
		fi; \
		rm -f "$$f"; \
	done

clean-docs:
	@echo "[clean] removing docs build artifacts"
	@rm -rf docs/.vitepress/cache docs/.vitepress/dist

clean-docs-deps:
	@echo "[clean] removing docs dependency installs"
	@rm -rf docs/node_modules

clean-tool-cache:
	@echo "[clean] removing preserved tool caches"
	@rm -rf $(TOOL_CACHE_DIRS)

distclean: clean clean-tool-cache clean-docs-deps
	@echo "[clean] distclean done"

install-renjin:
	@echo "[tool] installing Renjin $(RENJIN_VERSION)"
	@mkdir -p target/tmp/renjin-dist
	@curl -L "$(RENJIN_URL)" -o target/tmp/renjin-dist/renjin-$(RENJIN_VERSION).zip
	@rm -rf target/tmp/renjin-dist/renjin-$(RENJIN_VERSION)
	@unzip -q -o target/tmp/renjin-dist/renjin-$(RENJIN_VERSION).zip -d target/tmp/renjin-dist
	@chmod +x target/tmp/renjin-dist/renjin-$(RENJIN_VERSION)/bin/renjin
	@echo "[tool] installed target/tmp/renjin-dist/renjin-$(RENJIN_VERSION)/bin/renjin"

compiler-parallel-smoke:
	@./scripts/compiler_parallel_smoke.sh

library-package-suite:
	@bash ./scripts/library_package_suite.sh

optimizer-suite-legality:
	@bash ./scripts/optimizer_suite.sh legality

optimizer-suite-heavy:
	@bash ./scripts/optimizer_suite.sh heavy

optimizer-suite-all:
	@bash ./scripts/optimizer_suite.sh all

perf-gate:
	@bash ./scripts/perf_gate.sh

recommended-package-coverage:
	@bash ./scripts/recommended_package_coverage.sh

test-tier0:
	@bash ./scripts/test_tier.sh tier0

test-tier1:
	@bash ./scripts/test_tier.sh tier1

test-tier2-main:
	@bash ./scripts/test_tier.sh tier2-main

test-tier2-differential:
	@bash ./scripts/test_tier.sh tier2-differential
