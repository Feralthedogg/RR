.PHONY: clean clean-target clean-fuzz clean-native clean-artifacts clean-r

clean: clean-target clean-fuzz clean-native clean-artifacts clean-r
	@echo "[clean] done"

clean-target:
	@echo "[clean] removing Rust build artifacts"
	@cargo clean

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
		if git ls-files --error-unmatch "$$f" >/dev/null 2>&1; then \
			continue; \
		fi; \
		rm -f "$$f"; \
	done
