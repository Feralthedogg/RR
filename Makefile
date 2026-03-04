.PHONY: clean clean-target clean-fuzz clean-native clean-r

clean: clean-target clean-fuzz clean-native clean-r
	@echo "[clean] done"

clean-target:
	@echo "[clean] removing Rust build artifacts"
	@cargo clean
	@rm -rf target

clean-fuzz:
	@echo "[clean] removing fuzz artifacts"
	@rm -rf fuzz/artifacts
	@rm -rf fuzz/coverage
	@rm -rf fuzz/target
	@rm -f fuzz/*.profraw fuzz/*.profdata fuzz/*.log

clean-native:
	@echo "[clean] removing native build outputs"
	@rm -f native/*.so native/*.dylib native/*.dll native/*.o native/*.a

clean-r:
	@echo "[clean] removing untracked generated R files"
	@find . -type f \( -name '*.R' -o -name '*.r' -o -name '*.gen.R' \) -print0 | \
	while IFS= read -r -d '' f; do \
		if git ls-files --error-unmatch "$$f" >/dev/null 2>&1; then \
			continue; \
		fi; \
		rm -f "$$f"; \
	done
