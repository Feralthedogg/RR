.PHONY: clean clean-target clean-fuzz clean-native clean-artifacts clean-r clean-tool-cache distclean install-renjin

RENJIN_VERSION := 3.5-beta76
RENJIN_URL := https://www.renjin.org/repo/dist/renjin-$(RENJIN_VERSION).zip
TOOL_CACHE_DIRS := target/tmp/bench-python target/tmp/renjin-dist

clean: clean-target clean-fuzz clean-native clean-artifacts clean-r
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
	@find . -type f \( \
		-name 'rr_*.png' -o \
		-name 'rr_*.pdf' -o \
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

clean-tool-cache:
	@echo "[clean] removing preserved tool caches"
	@rm -rf $(TOOL_CACHE_DIRS)

distclean: clean clean-tool-cache
	@echo "[clean] distclean done"

install-renjin:
	@echo "[tool] installing Renjin $(RENJIN_VERSION)"
	@mkdir -p target/tmp/renjin-dist
	@curl -L "$(RENJIN_URL)" -o target/tmp/renjin-dist/renjin-$(RENJIN_VERSION).zip
	@rm -rf target/tmp/renjin-dist/renjin-$(RENJIN_VERSION)
	@unzip -q -o target/tmp/renjin-dist/renjin-$(RENJIN_VERSION).zip -d target/tmp/renjin-dist
	@chmod +x target/tmp/renjin-dist/renjin-$(RENJIN_VERSION)/bin/renjin
	@echo "[tool] installed target/tmp/renjin-dist/renjin-$(RENJIN_VERSION)/bin/renjin"
